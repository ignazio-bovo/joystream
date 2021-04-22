import { Api } from '../Api'
import BN from 'bn.js'
import { assert } from 'chai'
import { BaseFixture } from '../Fixture'
import Debugger from 'debug'
import { QueryNodeApi } from '../QueryNodeApi'
import {
  ApplicationFormQuestionType,
  AppliedOnOpeningEvent,
  EventType,
  OpeningAddedEvent,
  OpeningFilledEvent,
  WorkingGroupApplication,
  WorkingGroupOpening,
  WorkingGroupOpeningType,
  Worker,
  ApplicationWithdrawnEvent,
  OpeningCanceledEvent,
  StatusTextChangedEvent,
  UpcomingWorkingGroupOpening,
  WorkingGroupOpeningMetadata,
  WorkingGroup,
  WorkingGroupMetadata as QWorkingGroupMetadata,
} from '../QueryNodeApiSchema.generated'
import {
  AddUpcomingOpening,
  ApplicationMetadata,
  OpeningMetadata,
  RemoveUpcomingOpening,
  UpcomingOpeningMetadata,
  WorkingGroupMetadataAction,
  WorkingGroupMetadata,
  SetGroupMetadata,
} from '@joystream/metadata-protobuf'
import {
  WorkingGroupModuleName,
  MemberContext,
  AppliedOnOpeningEventDetails,
  OpeningAddedEventDetails,
  OpeningFilledEventDetails,
  lockIdByWorkingGroup,
  EventDetails,
} from '../types'
import { Application, ApplicationId, Opening, OpeningId } from '@joystream/types/working-group'
import { Utils } from '../utils'
import _ from 'lodash'
import { SubmittableExtrinsic } from '@polkadot/api/types'
import { JoyBTreeSet } from '@joystream/types/common'
import { registry } from '@joystream/types'

// TODO: Fetch from runtime when possible!
const MIN_APPLICATION_STAKE = new BN(2000)
const MIN_USTANKING_PERIOD = 43201
export const LEADER_OPENING_STAKE = new BN(2000)

export type OpeningParams = {
  stake: BN
  unstakingPeriod: number
  reward: BN
  metadata: OpeningMetadata.AsObject
}

const queryNodeQuestionTypeToMetadataQuestionType = (type: ApplicationFormQuestionType) => {
  if (type === ApplicationFormQuestionType.Text) {
    return OpeningMetadata.ApplicationFormQuestion.InputType.TEXT
  }

  return OpeningMetadata.ApplicationFormQuestion.InputType.TEXTAREA
}

abstract class BaseCreateOpeningFixture extends BaseFixture {
  protected query: QueryNodeApi
  protected group: WorkingGroupModuleName
  protected openingParams: OpeningParams

  public constructor(
    api: Api,
    query: QueryNodeApi,
    group: WorkingGroupModuleName,
    openingParams?: Partial<OpeningParams>
  ) {
    super(api)
    this.query = query
    this.group = group
    this.openingParams = _.merge(this.defaultOpeningParams, openingParams)
  }

  private defaultOpeningParams: OpeningParams = {
    stake: MIN_APPLICATION_STAKE,
    unstakingPeriod: MIN_USTANKING_PERIOD,
    reward: new BN(10),
    metadata: {
      shortDescription: 'Test opening',
      description: '# Test opening',
      expectedEndingTimestamp: Date.now() + 60,
      hiringLimit: 1,
      applicationDetails: '- This is automatically created opening, do not apply!',
      applicationFormQuestionsList: [
        { question: 'Question 1?', type: OpeningMetadata.ApplicationFormQuestion.InputType.TEXT },
        { question: 'Question 2?', type: OpeningMetadata.ApplicationFormQuestion.InputType.TEXTAREA },
      ],
    },
  }

  public getDefaultOpeningParams(): OpeningParams {
    return this.defaultOpeningParams
  }

  protected getMetadata(): OpeningMetadata {
    const metadataObj = this.openingParams.metadata as Required<OpeningMetadata.AsObject>
    const metadata = new OpeningMetadata()
    metadata.setShortDescription(metadataObj.shortDescription)
    metadata.setDescription(metadataObj.description)
    metadata.setExpectedEndingTimestamp(metadataObj.expectedEndingTimestamp)
    metadata.setHiringLimit(metadataObj.hiringLimit)
    metadata.setApplicationDetails(metadataObj.applicationDetails)
    metadataObj.applicationFormQuestionsList.forEach(({ question, type }) => {
      const applicationFormQuestion = new OpeningMetadata.ApplicationFormQuestion()
      applicationFormQuestion.setQuestion(question!)
      applicationFormQuestion.setType(type!)
      metadata.addApplicationFormQuestions(applicationFormQuestion)
    })

    return metadata
  }

  protected assertQueriedOpeningMetadataIsValid(qOpeningMeta: WorkingGroupOpeningMetadata) {
    assert.equal(qOpeningMeta.shortDescription, this.openingParams.metadata.shortDescription)
    assert.equal(qOpeningMeta.description, this.openingParams.metadata.description)
    assert.equal(new Date(qOpeningMeta.expectedEnding).getTime(), this.openingParams.metadata.expectedEndingTimestamp)
    assert.equal(qOpeningMeta.hiringLimit, this.openingParams.metadata.hiringLimit)
    assert.equal(qOpeningMeta.applicationDetails, this.openingParams.metadata.applicationDetails)
    assert.deepEqual(
      qOpeningMeta.applicationFormQuestions
        .sort((a, b) => a.index - b.index)
        .map(({ question, type }) => ({
          question,
          type: queryNodeQuestionTypeToMetadataQuestionType(type),
        })),
      this.openingParams.metadata.applicationFormQuestionsList
    )
  }

  abstract execute(): Promise<void>
}
export class CreateOpeningFixture extends BaseCreateOpeningFixture {
  private debug: Debugger.Debugger
  private asSudo: boolean

  private event?: OpeningAddedEventDetails
  private tx?: SubmittableExtrinsic<'promise'>

  public constructor(
    api: Api,
    query: QueryNodeApi,
    group: WorkingGroupModuleName,
    openingParams?: Partial<OpeningParams>,
    asSudo = false
  ) {
    super(api, query, group, openingParams)
    this.debug = Debugger(`fixture:CreateOpeningFixture:${group}`)
    this.asSudo = asSudo
  }

  public getCreatedOpeningId(): OpeningId {
    if (!this.event) {
      throw new Error('Trying to get created opening id before it was created!')
    }
    return this.event.openingId
  }

  private assertOpeningMatchQueriedResult(
    eventDetails: OpeningAddedEventDetails,
    qOpening?: WorkingGroupOpening | null
  ) {
    if (!qOpening) {
      throw new Error('Query node: Opening not found')
    }
    assert.equal(qOpening.runtimeId, eventDetails.openingId.toNumber())
    assert.equal(qOpening.createdAtBlock.number, eventDetails.blockNumber)
    assert.equal(qOpening.group.name, this.group)
    assert.equal(qOpening.rewardPerBlock, this.openingParams.reward.toString())
    assert.equal(qOpening.type, this.asSudo ? WorkingGroupOpeningType.Leader : WorkingGroupOpeningType.Regular)
    assert.equal(qOpening.status.__typename, 'OpeningStatusOpen')
    assert.equal(qOpening.stakeAmount, this.openingParams.stake.toString())
    assert.equal(qOpening.unstakingPeriod, this.openingParams.unstakingPeriod)
    // Metadata
    this.assertQueriedOpeningMetadataIsValid(qOpening.metadata)
  }

  private assertQueriedOpeningAddedEventIsValid(
    eventDetails: OpeningAddedEventDetails,
    txHash: string,
    qEvent?: OpeningAddedEvent
  ) {
    if (!qEvent) {
      throw new Error('Query node: OpeningAdded event not found')
    }
    assert.equal(qEvent.event.inExtrinsic, txHash)
    assert.equal(qEvent.event.type, EventType.OpeningAdded)
    assert.equal(qEvent.group.name, this.group)
    assert.equal(qEvent.opening.runtimeId, eventDetails.openingId.toNumber())
  }

  async execute(): Promise<void> {
    const account = this.asSudo
      ? await (await this.api.query.sudo.key()).toString()
      : await this.api.getLeadRoleKey(this.group)
    this.tx = this.api.tx[this.group].addOpening(
      Utils.metadataToBytes(this.getMetadata()),
      this.asSudo ? 'Leader' : 'Regular',
      { stake_amount: this.openingParams.stake, leaving_unstaking_period: this.openingParams.unstakingPeriod },
      this.openingParams.reward
    )
    if (this.asSudo) {
      this.tx = this.api.tx.sudo.sudo(this.tx)
    }
    const txFee = await this.api.estimateTxFee(this.tx, account)
    await this.api.treasuryTransferBalance(account, txFee)
    const result = await this.api.signAndSend(this.tx, account)
    this.event = await this.api.retrieveOpeningAddedEventDetails(result, this.group)

    this.debug(`Opening created (id: ${this.event.openingId.toString()})`)
  }

  async runQueryNodeChecks(): Promise<void> {
    await super.runQueryNodeChecks()
    const eventDetails = this.event!
    const tx = this.tx!
    // Query the opening
    await this.query.tryQueryWithTimeout(
      () => this.query.getOpeningById(eventDetails.openingId, this.group),
      (r) => this.assertOpeningMatchQueriedResult(eventDetails, r.data.workingGroupOpeningByUniqueInput)
    )
    // Query the event
    const qOpeningAddedEvent = await this.query.getOpeningAddedEvent(
      eventDetails.blockNumber,
      eventDetails.indexInBlock
    )
    this.assertQueriedOpeningAddedEventIsValid(eventDetails, tx.hash.toString(), qOpeningAddedEvent)
  }
}

export class ApplyOnOpeningHappyCaseFixture extends BaseFixture {
  private query: QueryNodeApi
  private group: WorkingGroupModuleName
  private debug: Debugger.Debugger
  private applicant: MemberContext
  private stakingAccount: string
  private openingId: OpeningId
  private openingMetadata: OpeningMetadata.AsObject

  private opening?: Opening
  private event?: AppliedOnOpeningEventDetails
  private tx?: SubmittableExtrinsic<'promise'>

  public constructor(
    api: Api,
    query: QueryNodeApi,
    group: WorkingGroupModuleName,
    applicant: MemberContext,
    stakingAccount: string,
    openingId: OpeningId,
    openingMetadata: OpeningMetadata.AsObject
  ) {
    super(api)
    this.query = query
    this.debug = Debugger(`fixture:ApplyOnOpeningHappyCaseFixture:${group}`)
    this.group = group
    this.applicant = applicant
    this.stakingAccount = stakingAccount
    this.openingId = openingId
    this.openingMetadata = openingMetadata
  }

  public getCreatedApplicationId(): ApplicationId {
    if (!this.event) {
      throw new Error('Trying to get created application id before the application was created!')
    }
    return this.event.applicationId
  }

  private getMetadata(): ApplicationMetadata {
    const metadata = new ApplicationMetadata()
    this.openingMetadata.applicationFormQuestionsList.forEach((question, i) => {
      metadata.addAnswers(`Answer ${i}`)
    })
    return metadata
  }

  private assertApplicationMatchQueriedResult(
    eventDetails: AppliedOnOpeningEventDetails,
    qApplication?: WorkingGroupApplication | null
  ) {
    if (!qApplication) {
      throw new Error('Application not found')
    }
    assert.equal(qApplication.runtimeId, eventDetails.applicationId.toNumber())
    assert.equal(qApplication.createdAtBlock.number, eventDetails.blockNumber)
    assert.equal(qApplication.opening.runtimeId, this.openingId.toNumber())
    assert.equal(qApplication.applicant.id, this.applicant.memberId.toString())
    assert.equal(qApplication.roleAccount, this.applicant.account)
    assert.equal(qApplication.rewardAccount, this.applicant.account)
    assert.equal(qApplication.stakingAccount, this.stakingAccount)
    assert.equal(qApplication.status.__typename, 'ApplicationStatusPending')
    assert.equal(qApplication.stake, eventDetails.params.stake_parameters.stake)

    const applicationMetadata = this.getMetadata()
    assert.deepEqual(
      qApplication.answers.map(({ question: { question }, answer }) => ({ question, answer })),
      this.openingMetadata.applicationFormQuestionsList.map(({ question }, index) => ({
        question,
        answer: applicationMetadata.getAnswersList()[index],
      }))
    )
  }

  private assertQueriedOpeningAddedEventIsValid(
    eventDetails: AppliedOnOpeningEventDetails,
    txHash: string,
    qEvent?: AppliedOnOpeningEvent
  ) {
    if (!qEvent) {
      throw new Error('Query node: AppliedOnOpening event not found')
    }
    assert.equal(qEvent.event.inExtrinsic, txHash)
    assert.equal(qEvent.event.type, EventType.AppliedOnOpening)
    assert.equal(qEvent.group.name, this.group)
    assert.equal(qEvent.opening.runtimeId, this.openingId.toNumber())
    assert.equal(qEvent.application.runtimeId, eventDetails.applicationId.toNumber())
  }

  async execute(): Promise<void> {
    this.opening = await this.api.getOpening(this.group, this.openingId)
    const stake = this.opening.stake_policy.stake_amount
    const stakingAccountBalance = await this.api.getBalance(this.stakingAccount)
    assert.isAbove(stakingAccountBalance.toNumber(), stake.toNumber())

    this.tx = this.api.tx[this.group].applyOnOpening({
      member_id: this.applicant.memberId,
      description: Utils.metadataToBytes(this.getMetadata()),
      opening_id: this.openingId,
      reward_account_id: this.applicant.account,
      role_account_id: this.applicant.account,
      stake_parameters: {
        stake,
        staking_account_id: this.stakingAccount,
      },
    })
    const txFee = await this.api.estimateTxFee(this.tx, this.applicant.account)
    await this.api.treasuryTransferBalance(this.applicant.account, txFee)
    const result = await this.api.signAndSend(this.tx, this.applicant.account)
    this.event = await this.api.retrieveAppliedOnOpeningEventDetails(result, this.group)

    this.debug(`Application submitted (id: ${this.event.applicationId.toString()})`)
  }

  async runQueryNodeChecks(): Promise<void> {
    await super.runQueryNodeChecks()
    const eventDetails = this.event!
    const tx = this.tx!
    // Query the application
    await this.query.tryQueryWithTimeout(
      () => this.query.getApplicationById(eventDetails.applicationId, this.group),
      (r) => this.assertApplicationMatchQueriedResult(eventDetails, r.data.workingGroupApplicationByUniqueInput)
    )
    // Query the event
    const qAppliedOnOpeningEvent = await this.query.getAppliedOnOpeningEvent(
      eventDetails.blockNumber,
      eventDetails.indexInBlock
    )
    this.assertQueriedOpeningAddedEventIsValid(eventDetails, tx.hash.toString(), qAppliedOnOpeningEvent)
  }
}

export class SudoFillLeadOpeningFixture extends BaseFixture {
  private query: QueryNodeApi
  private group: WorkingGroupModuleName
  private debug: Debugger.Debugger
  private openingId: OpeningId
  private acceptedApplicationIds: ApplicationId[]

  private acceptedApplications?: Application[]
  private applicationStakes?: BN[]
  private tx?: SubmittableExtrinsic<'promise'>
  private event?: OpeningFilledEventDetails

  public constructor(
    api: Api,
    query: QueryNodeApi,
    group: WorkingGroupModuleName,
    openingId: OpeningId,
    acceptedApplicationIds: ApplicationId[]
  ) {
    super(api)
    this.query = query
    this.debug = Debugger(`fixture:SudoFillLeadOpeningFixture:${group}`)
    this.group = group
    this.openingId = openingId
    this.acceptedApplicationIds = acceptedApplicationIds
  }

  async execute() {
    // Query the applications data
    this.acceptedApplications = await this.api.query[this.group].applicationById.multi(this.acceptedApplicationIds)
    this.applicationStakes = await Promise.all(
      this.acceptedApplications.map((a) =>
        this.api.getStakedBalance(a.staking_account_id, lockIdByWorkingGroup[this.group])
      )
    )
    // Fill the opening
    this.tx = this.api.tx.sudo.sudo(
      this.api.tx[this.group].fillOpening(
        this.openingId,
        new (JoyBTreeSet(ApplicationId))(registry, this.acceptedApplicationIds)
      )
    )
    const sudoKey = await this.api.query.sudo.key()
    const result = await this.api.signAndSend(this.tx, sudoKey)
    this.event = await this.api.retrieveOpeningFilledEventDetails(result, this.group)
  }

  private assertQueriedOpeningFilledEventIsValid(
    eventDetails: OpeningFilledEventDetails,
    txHash: string,
    qEvent?: OpeningFilledEvent
  ) {
    if (!qEvent) {
      throw new Error('Query node: OpeningFilledEvent not found')
    }
    assert.equal(qEvent.event.inExtrinsic, txHash)
    assert.equal(qEvent.event.type, EventType.OpeningFilled)
    assert.equal(qEvent.opening.runtimeId, this.openingId.toNumber())
    assert.equal(qEvent.group.name, this.group)
    this.acceptedApplicationIds.forEach((acceptedApplId, i) => {
      // Cannot use "applicationIdToWorkerIdMap.get" here,
      // it only works if the passed instance is identical to BTreeMap key instance (=== instead of .eq)
      const [, workerId] =
        Array.from(eventDetails.applicationIdToWorkerIdMap.entries()).find(([applicationId]) =>
          applicationId.eq(acceptedApplId)
        ) || []
      if (!workerId) {
        throw new Error(`WorkerId for application id ${acceptedApplId.toString()} not found in OpeningFilled event!`)
      }
      const qWorker = qEvent.workersHired.find((w) => w.runtimeId === workerId.toNumber())
      if (!qWorker) {
        throw new Error(`Query node: Worker not found in OpeningFilled.hiredWorkers (id: ${workerId.toString()})`)
      }
      this.assertHiredWorkerIsValid(
        eventDetails,
        this.acceptedApplicationIds[i],
        this.acceptedApplications![i],
        this.applicationStakes![i],
        qWorker
      )
    })
  }

  private assertHiredWorkerIsValid(
    eventDetails: OpeningFilledEventDetails,
    applicationId: ApplicationId,
    application: Application,
    applicationStake: BN,
    qWorker: Worker
  ) {
    assert.equal(qWorker.group.name, this.group)
    assert.equal(qWorker.membership.id, application.member_id.toString())
    assert.equal(qWorker.roleAccount, application.role_account_id.toString())
    assert.equal(qWorker.rewardAccount, application.reward_account_id.toString())
    assert.equal(qWorker.stakeAccount, application.staking_account_id.toString())
    assert.equal(qWorker.status.__typename, 'WorkerStatusActive')
    assert.equal(qWorker.isLead, true)
    assert.equal(qWorker.stake, applicationStake.toString())
    assert.equal(qWorker.hiredAtBlock.number, eventDetails.blockNumber)
    assert.equal(qWorker.application.runtimeId, applicationId.toNumber())
  }

  async runQueryNodeChecks(): Promise<void> {
    await super.runQueryNodeChecks()
    const eventDetails = this.event!
    const tx = this.tx!
    // Query the event and check event + hiredWorkers
    const qEvent = (await this.query.tryQueryWithTimeout(
      () => this.query.getOpeningFilledEvent(eventDetails.blockNumber, eventDetails.indexInBlock),
      (qEvent) => this.assertQueriedOpeningFilledEventIsValid(eventDetails, tx.hash.toString(), qEvent)
    )) as OpeningFilledEvent

    // Check opening status
    const {
      data: { workingGroupOpeningByUniqueInput: qOpening },
    } = await this.query.getOpeningById(this.openingId, this.group)
    if (!qOpening) {
      throw new Error(`Query node: Opening ${this.openingId.toString()} not found!`)
    }
    assert.equal(qOpening.status.__typename, 'OpeningStatusFilled')
    if (qOpening.status.__typename === 'OpeningStatusFilled') {
      assert.equal(qOpening.status.openingFilledEventId, qEvent.id)
    }

    // Check application statuses
    const acceptedQApplications = this.acceptedApplicationIds.map((id) => {
      const qApplication = qOpening.applications.find((a) => a.runtimeId === id.toNumber())
      if (!qApplication) {
        throw new Error(`Query node: Application not found by id ${id.toString()} in opening ${qOpening.id}`)
      }
      assert.equal(qApplication.status.__typename, 'ApplicationStatusAccepted')
      if (qApplication.status.__typename === 'ApplicationStatusAccepted') {
        assert.equal(qApplication.status.openingFilledEventId, qEvent.id)
      }
      return qApplication
    })

    qOpening.applications
      .filter((a) => !acceptedQApplications.some((acceptedQA) => acceptedQA.id === a.id))
      .forEach((qApplication) => {
        assert.equal(qApplication.status.__typename, 'ApplicationStatusRejected')
        if (qApplication.status.__typename === 'ApplicationStatusRejected') {
          assert.equal(qApplication.status.openingFilledEventId, qEvent.id)
        }
      })

    // Check working group lead
    if (!qOpening.group.leader) {
      throw new Error('Query node: Group leader not set!')
    }
    assert.equal(qOpening.group.leader.runtimeId, qEvent.workersHired[0].runtimeId)
  }
}

export class WithdrawApplicationsFixture extends BaseFixture {
  private query: QueryNodeApi
  private group: WorkingGroupModuleName
  private debug: Debugger.Debugger
  private applicationIds: ApplicationId[]
  private accounts: string[]

  private txs: SubmittableExtrinsic<'promise'>[] = []
  private events: EventDetails[] = []

  public constructor(
    api: Api,
    query: QueryNodeApi,
    group: WorkingGroupModuleName,
    accounts: string[],
    applicationIds: ApplicationId[]
  ) {
    super(api)
    this.query = query
    this.debug = Debugger(`fixture:WithdrawApplicationsFixture:${group}`)
    this.group = group
    this.accounts = accounts
    this.applicationIds = applicationIds
  }

  async execute() {
    this.txs = this.applicationIds.map((id) => this.api.tx[this.group].withdrawApplication(id))
    const txFee = await this.api.estimateTxFee(this.txs[0], this.accounts[0])
    await Promise.all(this.accounts.map((a) => this.api.treasuryTransferBalance(a, txFee)))
    const results = await Promise.all(this.txs.map((tx, i) => this.api.signAndSend(tx, this.accounts[i])))
    this.events = await Promise.all(
      results.map((r) => this.api.retrieveWorkingGroupsEventDetails(r, this.group, 'ApplicationWithdrawn'))
    )
  }

  private assertQueriedApplicationWithdrawnEventIsValid(
    applicationId: ApplicationId,
    txHash: string,
    qEvent?: ApplicationWithdrawnEvent
  ) {
    if (!qEvent) {
      throw new Error('Query node: ApplicationWithdrawnEvent not found!')
    }
    assert.equal(qEvent.event.inExtrinsic, txHash)
    assert.equal(qEvent.event.type, EventType.ApplicationWithdrawn)
    assert.equal(qEvent.application.runtimeId, applicationId.toNumber())
    assert.equal(qEvent.group.name, this.group)
  }

  private assertApplicationStatusIsValid(
    qEvent: ApplicationWithdrawnEvent,
    qApplication?: WorkingGroupApplication | null
  ) {
    if (!qApplication) {
      throw new Error('Query node: Application not found!')
    }
    assert.equal(qApplication.status.__typename, 'ApplicationStatusWithdrawn')
    if (qApplication.status.__typename === 'ApplicationStatusWithdrawn') {
      assert.equal(qApplication.status.applicationWithdrawnEventId, qEvent.id)
    }
  }

  async runQueryNodeChecks(): Promise<void> {
    await super.runQueryNodeChecks()

    // Query the evens
    const qEvents = (await Promise.all(
      this.events.map((eventDetails, i) =>
        this.query.tryQueryWithTimeout(
          () => this.query.getApplicationWithdrawnEvent(eventDetails.blockNumber, eventDetails.indexInBlock),
          (qEvent) =>
            this.assertQueriedApplicationWithdrawnEventIsValid(
              this.applicationIds[i],
              this.txs[i].hash.toString(),
              qEvent
            )
        )
      )
    )) as ApplicationWithdrawnEvent[]

    // Check application statuses
    await Promise.all(
      this.applicationIds.map(async (id, i) => {
        const {
          data: { workingGroupApplicationByUniqueInput: qApplication },
        } = await this.query.getApplicationById(id, this.group)
        this.assertApplicationStatusIsValid(qEvents[i], qApplication)
      })
    )
  }
}

export class CancelOpeningFixture extends BaseFixture {
  private query: QueryNodeApi
  private group: WorkingGroupModuleName
  private debug: Debugger.Debugger
  private openingId: OpeningId

  private tx?: SubmittableExtrinsic<'promise'>
  private event?: EventDetails

  public constructor(api: Api, query: QueryNodeApi, group: WorkingGroupModuleName, openingId: OpeningId) {
    super(api)
    this.query = query
    this.debug = Debugger(`fixture:CancelOpeningFixture:${group}`)
    this.group = group
    this.openingId = openingId
  }

  async execute() {
    const account = await this.api.getLeadRoleKey(this.group)
    this.tx = this.api.tx[this.group].cancelOpening(this.openingId)
    const txFee = await this.api.estimateTxFee(this.tx, account)
    await this.api.treasuryTransferBalance(account, txFee)
    const result = await this.api.signAndSend(this.tx, account)
    this.event = await this.api.retrieveWorkingGroupsEventDetails(result, this.group, 'OpeningCanceled')
  }

  private assertQueriedOpeningIsValid(qEvent: OpeningCanceledEvent, qOpening?: WorkingGroupOpening | null) {
    if (!qOpening) {
      throw new Error('Query node: Opening not found!')
    }
    assert.equal(qOpening.status.__typename, 'OpeningStatusCancelled')
    if (qOpening.status.__typename === 'OpeningStatusCancelled') {
      assert.equal(qOpening.status.openingCancelledEventId, qEvent.id)
    }
    qOpening.applications.forEach((a) => this.assertApplicationStatusIsValid(qEvent, a))
  }

  private assertApplicationStatusIsValid(qEvent: OpeningCanceledEvent, qApplication: WorkingGroupApplication) {
    // It's possible that some of the applications have been withdrawn
    assert.oneOf(qApplication.status.__typename, ['ApplicationStatusWithdrawn', 'ApplicationStatusCancelled'])
    if (qApplication.status.__typename === 'ApplicationStatusCancelled') {
      assert.equal(qApplication.status.openingCancelledEventId, qEvent.id)
    }
  }

  private assertQueriedOpeningCancelledEventIsValid(txHash: string, qEvent?: OpeningCanceledEvent) {
    if (!qEvent) {
      throw new Error('Query node: OpeningCancelledEvent not found!')
    }
    assert.equal(qEvent.event.inExtrinsic, txHash)
    assert.equal(qEvent.event.type, EventType.OpeningCanceled)
    assert.equal(qEvent.group.name, this.group)
    assert.equal(qEvent.opening.runtimeId, this.openingId.toNumber())
  }

  async runQueryNodeChecks(): Promise<void> {
    await super.runQueryNodeChecks()
    const tx = this.tx!
    const event = this.event!
    const qEvent = (await this.query.tryQueryWithTimeout(
      () => this.query.getOpeningCancelledEvent(event.blockNumber, event.indexInBlock),
      (qEvent) => this.assertQueriedOpeningCancelledEventIsValid(tx.hash.toString(), qEvent)
    )) as OpeningCanceledEvent
    const {
      data: { workingGroupOpeningByUniqueInput: qOpening },
    } = await this.query.getOpeningById(this.openingId, this.group)
    this.assertQueriedOpeningIsValid(qEvent, qOpening)
  }
}
export class CreateUpcomingOpeningFixture extends BaseCreateOpeningFixture {
  private debug: Debugger.Debugger
  private expectedStartTs: number

  private tx?: SubmittableExtrinsic<'promise'>
  private event?: EventDetails
  private createdUpcomingOpeningId?: string

  public constructor(
    api: Api,
    query: QueryNodeApi,
    group: WorkingGroupModuleName,
    openingParams?: OpeningParams,
    expectedStartTs?: number
  ) {
    super(api, query, group, openingParams)
    this.debug = Debugger(`fixture:CreateUpcomingOpening:${group}`)
    this.expectedStartTs = expectedStartTs || Date.now() + 3600
  }

  public getCreatedUpcomingOpeningId() {
    if (!this.createdUpcomingOpeningId) {
      throw new Error('Trying to get created UpcomingOpening id before it is known')
    }
    return this.createdUpcomingOpeningId
  }

  private getActionMetadata(): WorkingGroupMetadataAction {
    const actionMeta = new WorkingGroupMetadataAction()
    const addUpcomingOpeningMeta = new AddUpcomingOpening()

    const upcomingOpeningMeta = new UpcomingOpeningMetadata()
    const openingMeta = this.getMetadata()
    upcomingOpeningMeta.setMetadata(openingMeta)
    upcomingOpeningMeta.setExpectedStart(this.expectedStartTs)
    upcomingOpeningMeta.setMinApplicationStake(this.openingParams.stake.toNumber())
    upcomingOpeningMeta.setRewardPerBlock(this.openingParams.reward.toNumber())

    addUpcomingOpeningMeta.setMetadata(upcomingOpeningMeta)
    actionMeta.setAddUpcomingOpening(addUpcomingOpeningMeta)

    return actionMeta
  }

  async execute() {
    const account = await this.api.getLeadRoleKey(this.group)
    this.tx = this.api.tx[this.group].setStatusText(Utils.metadataToBytes(this.getActionMetadata()))
    const txFee = await this.api.estimateTxFee(this.tx, account)
    await this.api.treasuryTransferBalance(account, txFee)
    const result = await this.api.signAndSend(this.tx, account)
    this.event = await this.api.retrieveWorkingGroupsEventDetails(result, this.group, 'StatusTextChanged')
  }

  private assertQueriedUpcomingOpeningIsValid(
    eventDetails: EventDetails,
    qUpcomingOpening?: UpcomingWorkingGroupOpening | null
  ) {
    if (!qUpcomingOpening) {
      throw new Error('Query node: Upcoming opening not found!')
    }
    assert.equal(new Date(qUpcomingOpening.expectedStart).getTime(), this.expectedStartTs)
    assert.equal(qUpcomingOpening.group.name, this.group)
    assert.equal(qUpcomingOpening.rewardPerBlock, this.openingParams.reward.toString())
    assert.equal(qUpcomingOpening.stakeAmount, this.openingParams.stake.toString())
    assert.equal(qUpcomingOpening.createdAtBlock.number, eventDetails.blockNumber)
    this.assertQueriedOpeningMetadataIsValid(qUpcomingOpening.metadata)
  }

  private assertQueriedStatusTextChangedEventIsValid(txHash: string, qEvent?: StatusTextChangedEvent) {
    if (!qEvent) {
      throw new Error('Query node: StatusTextChangedEvent not found!')
    }
    assert.equal(qEvent.event.inExtrinsic, txHash)
    assert.equal(qEvent.event.type, EventType.StatusTextChanged)
    assert.equal(qEvent.group.name, this.group)
    assert.equal(qEvent.metadata, Utils.metadataToBytes(this.getActionMetadata()).toString())
    if (!qEvent.result) {
      throw new Error('StatusTextChangedEvent: No result found')
    }
    assert.equal(qEvent.result.__typename, 'UpcomingOpeningAdded')
  }

  async runQueryNodeChecks(): Promise<void> {
    await super.runQueryNodeChecks()
    const tx = this.tx!
    const event = this.event!
    // Query the event
    const qEvent = (await this.query.tryQueryWithTimeout(
      () => this.query.getStatusTextChangedEvent(event.blockNumber, event.indexInBlock),
      (qEvent) => this.assertQueriedStatusTextChangedEventIsValid(tx.hash.toString(), qEvent)
    )) as StatusTextChangedEvent
    // Query the opening
    const qUpcomingOpening = await this.query.getUpcomingOpeningByCreatedInEventId(qEvent.id)
    this.assertQueriedUpcomingOpeningIsValid(event, qUpcomingOpening)
    if (qEvent.result && qEvent.result.__typename === 'UpcomingOpeningAdded') {
      assert.equal(qEvent.result.upcomingOpeningId, qUpcomingOpening!.id)
    }
    this.createdUpcomingOpeningId = qUpcomingOpening!.id
  }
}

export class RemoveUpcomingOpeningFixture extends BaseFixture {
  private query: QueryNodeApi
  private group: WorkingGroupModuleName
  private upcomingOpeningId: string
  private debug: Debugger.Debugger

  private tx?: SubmittableExtrinsic<'promise'>
  private event?: EventDetails

  public constructor(api: Api, query: QueryNodeApi, group: WorkingGroupModuleName, upcomingOpeningId: string) {
    super(api)
    this.query = query
    this.group = group
    this.upcomingOpeningId = upcomingOpeningId
    this.debug = Debugger(`fixture:RemoveUpcomingOpeningFixture:${group}`)
  }

  private getActionMetadata(): WorkingGroupMetadataAction {
    const actionMeta = new WorkingGroupMetadataAction()
    const removeUpcomingOpeningMeta = new RemoveUpcomingOpening()
    removeUpcomingOpeningMeta.setId(this.upcomingOpeningId)
    actionMeta.setRemoveUpcomingOpening(removeUpcomingOpeningMeta)

    return actionMeta
  }

  async execute() {
    const account = await this.api.getLeadRoleKey(this.group)
    this.tx = this.api.tx[this.group].setStatusText(Utils.metadataToBytes(this.getActionMetadata()))
    const txFee = await this.api.estimateTxFee(this.tx, account)
    await this.api.treasuryTransferBalance(account, txFee)
    const result = await this.api.signAndSend(this.tx, account)
    this.event = await this.api.retrieveWorkingGroupsEventDetails(result, this.group, 'StatusTextChanged')
  }

  private assertQueriedStatusTextChangedEventIsValid(txHash: string, qEvent?: StatusTextChangedEvent) {
    if (!qEvent) {
      throw new Error('Query node: StatusTextChangedEvent not found!')
    }
    assert.equal(qEvent.event.inExtrinsic, txHash)
    assert.equal(qEvent.event.type, EventType.StatusTextChanged)
    assert.equal(qEvent.group.name, this.group)
    assert.equal(qEvent.metadata, Utils.metadataToBytes(this.getActionMetadata()).toString())
    if (!qEvent.result) {
      throw new Error('StatusTextChangedEvent: No result found')
    }
    assert.equal(qEvent.result.__typename, 'UpcomingOpeningRemoved')
    if (qEvent.result.__typename === 'UpcomingOpeningRemoved') {
      assert.equal(qEvent.result.upcomingOpeningId, this.upcomingOpeningId)
    }
  }

  async runQueryNodeChecks(): Promise<void> {
    await super.runQueryNodeChecks()
    const tx = this.tx!
    const event = this.event!
    // Query the event
    const qEvent = (await this.query.tryQueryWithTimeout(
      () => this.query.getStatusTextChangedEvent(event.blockNumber, event.indexInBlock),
      (qEvent) => this.assertQueriedStatusTextChangedEventIsValid(tx.hash.toString(), qEvent)
    )) as StatusTextChangedEvent
    // Query the opening and make sure it doesn't exist
    if (qEvent.result && qEvent.result.__typename === 'UpcomingOpeningRemoved') {
      const qUpcomingOpening = await this.query.getUpcomingOpeningByCreatedInEventId(qEvent.result.upcomingOpeningId)
      assert.isUndefined(qUpcomingOpening)
    }
  }
}

export class UpdateGroupStatusFixture extends BaseFixture {
  private query: QueryNodeApi
  private group: WorkingGroupModuleName
  private metadata: WorkingGroupMetadata.AsObject
  private debug: Debugger.Debugger

  private tx?: SubmittableExtrinsic<'promise'>
  private event?: EventDetails

  public constructor(
    api: Api,
    query: QueryNodeApi,
    group: WorkingGroupModuleName,
    metadata: WorkingGroupMetadata.AsObject
  ) {
    super(api)
    this.query = query
    this.group = group
    this.metadata = metadata
    this.debug = Debugger(`fixture:UpdateGroupStatusFixture:${group}`)
  }

  private getActionMetadata(): WorkingGroupMetadataAction {
    const actionMeta = new WorkingGroupMetadataAction()
    const setGroupMeta = new SetGroupMetadata()
    const newGroupMeta = new WorkingGroupMetadata()

    newGroupMeta.setAbout(this.metadata.about!)
    newGroupMeta.setDescription(this.metadata.description!)
    newGroupMeta.setStatus(this.metadata.status!)
    newGroupMeta.setStatusMessage(this.metadata.statusMessage!)

    setGroupMeta.setNewMetadata(newGroupMeta)
    actionMeta.setSetGroupMetadata(setGroupMeta)

    return actionMeta
  }

  async execute() {
    const account = await this.api.getLeadRoleKey(this.group)
    this.tx = this.api.tx[this.group].setStatusText(Utils.metadataToBytes(this.getActionMetadata()))
    const txFee = await this.api.estimateTxFee(this.tx, account)
    await this.api.treasuryTransferBalance(account, txFee)
    const result = await this.api.signAndSend(this.tx, account)
    this.event = await this.api.retrieveWorkingGroupsEventDetails(result, this.group, 'StatusTextChanged')
  }

  private assertQueriedStatusTextChangedEventIsValid(txHash: string, qEvent?: StatusTextChangedEvent) {
    if (!qEvent) {
      throw new Error('Query node: StatusTextChangedEvent not found!')
    }
    assert.equal(qEvent.event.inExtrinsic, txHash)
    assert.equal(qEvent.event.type, EventType.StatusTextChanged)
    assert.equal(qEvent.group.name, this.group)
    assert.equal(qEvent.metadata, Utils.metadataToBytes(this.getActionMetadata()).toString())
    if (!qEvent.result) {
      throw new Error('StatusTextChangedEvent: No result found')
    }
    assert.equal(qEvent.result.__typename, 'WorkingGroupMetadataSet')
  }

  private assertQueriedGroupIsValid(qGroup: WorkingGroup | undefined, qMeta: QWorkingGroupMetadata) {
    if (!qGroup) {
      throw new Error(`Query node: Group ${this.group} not found!`)
    }
    if (!qGroup.metadata) {
      throw new Error(`Query node: Group metadata is empty!`)
    }
    assert.equal(qGroup.metadata.id, qMeta.id)
  }

  private assertQueriedMetadataSnapshotsAreValid(
    eventDetails: EventDetails,
    beforeSnapshot?: QWorkingGroupMetadata,
    afterSnapshot?: QWorkingGroupMetadata
  ) {
    if (!afterSnapshot) {
      throw new Error('Query node: WorkingGroupMetadata snapshot not found!')
    }
    const expectedMeta = _.merge(this.metadata, beforeSnapshot)
    assert.equal(afterSnapshot.status, expectedMeta.status)
    assert.equal(afterSnapshot.statusMessage, expectedMeta.statusMessage)
    assert.equal(afterSnapshot.description, expectedMeta.description)
    assert.equal(afterSnapshot.about, expectedMeta.about)
    assert.equal(afterSnapshot.setAtBlock.number, eventDetails.blockNumber)
  }

  async runQueryNodeChecks(): Promise<void> {
    await super.runQueryNodeChecks()
    const tx = this.tx!
    const event = this.event!
    // Query & check the event
    const qEvent = (await this.query.tryQueryWithTimeout(
      () => this.query.getStatusTextChangedEvent(event.blockNumber, event.indexInBlock),
      (qEvent) => this.assertQueriedStatusTextChangedEventIsValid(tx.hash.toString(), qEvent)
    )) as StatusTextChangedEvent

    // Query & check the metadata snapshots
    const beforeSnapshot = await this.query.getGroupMetaSnapshot(event.blockTimestamp, 'lt')
    const afterSnapshot = await this.query.getGroupMetaSnapshot(event.blockTimestamp, 'eq')
    this.assertQueriedMetadataSnapshotsAreValid(event, beforeSnapshot, afterSnapshot)

    // Query & check the group
    const qGroup = await this.query.getWorkingGroup(this.group)
    this.assertQueriedGroupIsValid(qGroup, afterSnapshot!)

    // Check event relation
    if (qEvent.result && qEvent.result.__typename === 'WorkingGroupMetadataSet') {
      assert.equal(qEvent.result.metadataId, afterSnapshot!.id)
    }
  }
}
