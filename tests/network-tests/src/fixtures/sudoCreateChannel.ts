import { BaseFixture } from '../Fixture'
import { Api } from '../Api'
import { createTypeFromConstructor } from '@joystream/sumer-types'
import { ChannelCreationParameters } from '@joystream/types/content'
import { assert } from 'chai'


export class SudoCreateContent extends BaseFixture {

    constructor(
        api: Api,
    ) {
        super(api)
    }

    public async execute(): Promise<void> {

        // channel creation parameters
        const channelCreationParameters = createTypeFromConstructor(ChannelCreationParameters, {
            assets: null,
            meta: null,
        })

        // get next channel id
        const next_channel_id = await this.api.getNextChannelId();

        // create channel as sudo account (Alice)
        this.api.sudoCreateChannel(channelCreationParameters);

        // assert number of outstanding channels > 0
        assert(this.api.getChannelById(next_channel_id) !== null);

    }
}
