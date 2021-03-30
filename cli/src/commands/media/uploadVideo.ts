import VideoEntitySchema from '@joystream/cd-schemas/schemas/entities/VideoEntity.schema.json'
import VideoMediaEntitySchema from '@joystream/cd-schemas/schemas/entities/VideoMediaEntity.schema.json'
import { VideoEntity } from '@joystream/cd-schemas/types/entities/VideoEntity'
import { VideoMediaEntity } from '@joystream/cd-schemas/types/entities/VideoMediaEntity'
import { InputParser } from '@joystream/cd-schemas'
import { JSONSchema } from '@apidevtools/json-schema-ref-parser'
import { JsonSchemaPrompter } from '../../helpers/JsonSchemaPrompt'
import { flags } from '@oclif/command'
import fs from 'fs'
import ExitCodes from '../../ExitCodes'
import { ContentId } from '@joystream/types/storage'
import ipfsHash from 'ipfs-only-hash'
import { cli } from 'cli-ux'
import axios, { AxiosRequestConfig } from 'axios'
import { URL } from 'url'
import ipfsHttpClient from 'ipfs-http-client'
import first from 'it-first'
import last from 'it-last'
import toBuffer from 'it-to-buffer'
import ffprobeInstaller from '@ffprobe-installer/ffprobe'
import ffmpeg from 'fluent-ffmpeg'
import MediaCommandBase from '../../base/MediaCommandBase'
import { getInputJson, validateInput, IOFlags } from '../../helpers/InputOutput'

ffmpeg.setFfprobePath(ffprobeInstaller.path)

const DATA_OBJECT_TYPE_ID = 1
const MAX_FILE_SIZE = 2000 * 1024 * 1024

type VideoMetadata = {
  width?: number
  height?: number
  codecName?: string
  codecFullName?: string
  duration?: number
}

export default class UploadVideoCommand extends MediaCommandBase {
  static description = 'Upload a new Video to a channel (requires a membership).'
  static flags = {
    input: IOFlags.input,
    channel: flags.integer({
      char: 'c',
      required: false,
      description:
        'ID of the channel to assign the video to (if omitted - one of the owned channels can be selected from the list)',
    }),
    confirm: flags.boolean({ char: 'y', name: 'confirm', required: false, description: 'Confirm the provided input' }),
  }

  static args = [
    {
      name: 'filePath',
      required: true,
      description: 'Path to the media file to upload',
    },
  ]

  private createReadStreamWithProgressBar(filePath: string, barTitle: string, fileSize?: number) {
    // Progress CLI UX:
    // https://github.com/oclif/cli-ux#cliprogress
    // https://www.npmjs.com/package/cli-progress
    if (!fileSize) {
      fileSize = fs.statSync(filePath).size
    }
    const progress = cli.progress({ format: `${barTitle} | {bar} | {value}/{total} KB processed` })
    let processedKB = 0
    const fileSizeKB = Math.ceil(fileSize / 1024)
    progress.start(fileSizeKB, processedKB)
    return {
      fileStream: fs
        .createReadStream(filePath)
        .pause() // Explicitly pause to prevent switching to flowing mode (https://nodejs.org/api/stream.html#stream_event_data)
        .on('error', () => {
          progress.stop()
          this.error(`Error while trying to read data from: ${filePath}!`, {
            exit: ExitCodes.FsOperationFailed,
          })
        })
        .on('data', (data) => {
          processedKB += data.length / 1024
          progress.update(processedKB)
        })
        .on('end', () => {
          progress.update(fileSizeKB)
          progress.stop()
        }),
      progressBar: progress,
    }
  }

  private async calculateFileIpfsHash(filePath: string, fileSize: number): Promise<string> {
    const { fileStream } = this.createReadStreamWithProgressBar(filePath, 'Calculating file hash', fileSize)
    const hash: string = await ipfsHash.of(fileStream)

    return hash
  }

  private async getUploadUrl(storageProviderId: number, contentId: ContentId): Promise<string> {
    const endpoint = await this.getApi().storageProviderEndpoint(storageProviderId)
    return new URL(`asset/v0/${contentId.encode()}`, endpoint).toString()
  }

  private async getVideoMetadata(filePath: string): Promise<VideoMetadata | null> {
    let metadata: VideoMetadata | null = null
    const metadataPromise = new Promise<VideoMetadata>((resolve, reject) => {
      ffmpeg.ffprobe(filePath, (err, data) => {
        if (err) {
          reject(err)
          return
        }
        const videoStream = data.streams.find((s) => s.codec_type === 'video')
        if (videoStream) {
          resolve({
            width: videoStream.width,
            height: videoStream.height,
            codecName: videoStream.codec_name,
            codecFullName: videoStream.codec_long_name,
            duration: videoStream.duration !== undefined ? Math.ceil(Number(videoStream.duration)) || 0 : undefined,
          })
        } else {
          reject(new Error('No video stream found in file'))
        }
      })
    })

    try {
      metadata = await metadataPromise
    } catch (e) {
      const message = e.message || e
      this.warn(`Failed to get video metadata via ffprobe (${message})`)
    }

    return metadata
  }

  private async uploadVideo(filePath: string, fileSize: number, uploadUrl: string) {
    const { fileStream, progressBar } = this.createReadStreamWithProgressBar(filePath, 'Uploading', fileSize)
    fileStream.on('end', () => {
      cli.action.start('Waiting for the file to be processed...')
    })

    try {
      const config: AxiosRequestConfig = {
        headers: {
          'Content-Type': '', // https://github.com/Joystream/storage-node-joystream/issues/16
          'Content-Length': fileSize.toString(),
        },
        maxContentLength: MAX_FILE_SIZE,
        maxBodyLength: MAX_FILE_SIZE,
      }
      await axios.put(uploadUrl, fileStream, config)
      cli.action.stop()

      this.log('File uploaded!')
    } catch (e) {
      progressBar.stop()
      cli.action.stop()
      const msg = (e.response && e.response.data && e.response.data.message) || e.message || e
      this.error(`Unexpected error when trying to upload a file: ${msg}`, {
        exit: ExitCodes.ExternalInfrastructureError,
      })
    }
  }

  private async promptForVideoInput(
    channelId: number,
    fileSize: number,
    contentId: ContentId,
    videoMetadata: VideoMetadata | null
  ) {
    // Set the defaults
    const videoMediaDefaults: Partial<VideoMediaEntity> = {
      pixelWidth: videoMetadata?.width,
      pixelHeight: videoMetadata?.height,
    }
    const videoDefaults: Partial<VideoEntity> = {
      duration: videoMetadata?.duration,
      skippableIntroDuration: 0,
    }

    // Prompt for data
    const videoJsonSchema = (VideoEntitySchema as unknown) as JSONSchema
    const videoMediaJsonSchema = (VideoMediaEntitySchema as unknown) as JSONSchema

    const videoMediaPrompter = new JsonSchemaPrompter<VideoMediaEntity>(videoMediaJsonSchema, videoMediaDefaults)
    const videoPrompter = new JsonSchemaPrompter<VideoEntity>(videoJsonSchema, videoDefaults)

    // Prompt for the data
    const encodingSuggestion =
      videoMetadata && videoMetadata.codecFullName ? ` (suggested: ${videoMetadata.codecFullName})` : ''
    const encoding = await this.promptForEntityId(
      `Choose Video encoding${encodingSuggestion}`,
      'VideoMediaEncoding',
      'name'
    )
    const { pixelWidth, pixelHeight } = await videoMediaPrompter.promptMultipleProps(['pixelWidth', 'pixelHeight'])
    const language = await this.promptForEntityId('Choose Video language', 'Language', 'name')
    const category = await this.promptForEntityId('Choose Video category', 'ContentCategory', 'name')
    const videoProps = await videoPrompter.promptMultipleProps([
      'title',
      'description',
      'thumbnailUrl',
      'duration',
      'isPublic',
      'isExplicit',
      'hasMarketing',
      'skippableIntroDuration',
    ])

    const license = await videoPrompter.promptSingleProp('license', () => this.promptForNewLicense())
    const publishedBeforeJoystream = await videoPrompter.promptSingleProp('publishedBeforeJoystream', () =>
      this.promptForPublishedBeforeJoystream()
    )

    // Create final inputs
    const videoMediaInput: VideoMediaEntity = {
      encoding,
      pixelWidth,
      pixelHeight,
      size: fileSize,
      location: { new: { joystreamMediaLocation: { new: { dataObjectId: contentId.encode() } } } },
    }
    return {
      ...videoProps,
      channel: channelId,
      language,
      category,
      license,
      media: { new: videoMediaInput },
      publishedBeforeJoystream,
    }
  }

  private async getVideoInputFromFile(
    filePath: string,
    channelId: number,
    fileSize: number,
    contentId: ContentId,
    videoMetadata: VideoMetadata | null
  ) {
    let videoInput = await getInputJson<any>(filePath)
    if (typeof videoInput !== 'object' || videoInput === null) {
      this.error('Invalid input json - expected an object', { exit: ExitCodes.InvalidInput })
    }
    const videoMediaDefaults: Partial<VideoMediaEntity> = {
      pixelWidth: videoMetadata?.width,
      pixelHeight: videoMetadata?.height,
      size: fileSize,
    }
    const videoDefaults: Partial<VideoEntity> = {
      channel: channelId,
      duration: videoMetadata?.duration,
    }
    const inputVideoMedia =
      videoInput.media && typeof videoInput.media === 'object' && (videoInput.media as any).new
        ? (videoInput.media as any).new
        : {}
    videoInput = {
      ...videoDefaults,
      ...videoInput,
      media: {
        new: {
          ...videoMediaDefaults,
          ...inputVideoMedia,
          location: { new: { joystreamMediaLocation: { new: { dataObjectId: contentId.encode() } } } },
        },
      },
    }

    const videoJsonSchema = (VideoEntitySchema as unknown) as JSONSchema
    await validateInput(videoInput, videoJsonSchema)

    return videoInput as VideoEntity
  }

  async run() {
    const account = await this.getRequiredSelectedAccount()
    const memberId = await this.getRequiredMemberId()
    const actor = { Member: memberId }

    await this.requestAccountDecoding(account)

    const {
      args: { filePath },
      flags: { channel: inputChannelId, input, confirm },
    } = this.parse(UploadVideoCommand)

    // Basic file validation
    if (!fs.existsSync(filePath)) {
      this.error('File does not exist under provided path!', { exit: ExitCodes.FileNotFound })
    }

    const { size: fileSize } = fs.statSync(filePath)
    if (fileSize > MAX_FILE_SIZE) {
      this.error(`File size too large! Max. file size is: ${(MAX_FILE_SIZE / 1024 / 1024).toFixed(2)} MB`)
    }

    const videoMetadata = await this.getVideoMetadata(filePath)
    this.log('Video media file parameters established:', { ...(videoMetadata || {}), size: fileSize })

    // Check if any providers are available
    if (!(await this.getApi().isAnyProviderAvailable())) {
      this.error('No active storage providers available! Try again later...', {
        exit: ExitCodes.ActionCurrentlyUnavailable,
      })
    }

    // Start by prompting for a channel to make sure user has one available
    let channelId: number
    if (inputChannelId === undefined) {
      channelId = await this.promptForEntityId(
        'Select a channel to publish the video under',
        'Channel',
        'handle',
        memberId
      )
    } else {
      await this.getEntity(inputChannelId, 'Channel', memberId) // Validates if exists and belongs to member
      channelId = inputChannelId
    }

    // Calculate hash and create content id
    const contentId = ContentId.generate(this.getTypesRegistry())
    const ipfsCid = await this.calculateFileIpfsHash(filePath, fileSize)

    this.log('Video identification established:', {
      contentId: contentId.toString(),
      encodedContentId: contentId.encode(),
      ipfsHash: ipfsCid,
    })

    // Send dataDirectory.addContent extrinsic
    await this.sendAndFollowNamedTx(account, 'dataDirectory', 'addContent', [
      memberId,
      contentId,
      DATA_OBJECT_TYPE_ID,
      fileSize,
      ipfsCid,
    ])

    const dataObject = await this.getApi().dataByContentId(contentId)
    if (!dataObject) {
      this.error('Data object could not be retrieved from chain', { exit: ExitCodes.ApiError })
    }

    this.log('Data object:', dataObject.toJSON())

    // Get storage provider identity
    const storageProviderId = dataObject.liaison.toNumber()
    const ipnsIdentity = await this.getApi().ipnsIdentity(storageProviderId)

    if (!ipnsIdentity) {
      this.error('Storage provider IPNS identity could not be determined', { exit: ExitCodes.ApiError })
    }

    // Resolve upload url and upload the video
    const uploadUrl = await this.getUploadUrl(storageProviderId, contentId)
    this.log('Resolved upload url:', uploadUrl)

    await this.uploadVideo(filePath, fileSize, uploadUrl)

    // No input, create prompting helpers
    const videoInput = input
      ? await this.getVideoInputFromFile(input, channelId, fileSize, contentId, videoMetadata)
      : await this.promptForVideoInput(channelId, fileSize, contentId, videoMetadata)

    this.jsonPrettyPrint(JSON.stringify(videoInput))

    if (!confirm) {
      await this.requireConfirmation('Do you confirm the provided input?', true)
    }

    // Parse inputs into operations and send final extrinsic
    const inputParser = InputParser.createWithKnownSchemas(this.getOriginalApi(), [
      {
        className: 'Video',
        entries: [videoInput],
      },
    ])
    const operations = await inputParser.getEntityBatchOperations()
    await this.sendAndFollowNamedTx(account, 'contentDirectory', 'transaction', [actor, operations])
  }
}
