import { gql, ApolloClient, ApolloQueryResult, NormalizedCacheObject } from '@apollo/client'
import { MemberId } from '@joystream/types/common'
import {
  ApplicationWithdrawnEvent,
  AppliedOnOpeningEvent,
  InitialInvitationBalanceUpdatedEvent,
  InitialInvitationCountUpdatedEvent,
  MembershipPriceUpdatedEvent,
  MembershipSystemSnapshot,
  OpeningAddedEvent,
  OpeningCanceledEvent,
  OpeningFilledEvent,
  Query,
  ReferralCutUpdatedEvent,
  StatusTextChangedEvent,
  UpcomingWorkingGroupOpening,
  WorkingGroup,
  WorkingGroupMetadata,
} from './QueryNodeApiSchema.generated'
import Debugger from 'debug'
import { ApplicationId, OpeningId } from '@joystream/types/working-group'
import { WorkingGroupModuleName } from './types'

const EVENT_GENERIC_FIELDS = `
  id
  event {
    inBlock {
      number
      timestamp
      network
    }
    inExtrinsic
    indexInBlock
    type
  }
`

export class QueryNodeApi {
  private readonly queryNodeProvider: ApolloClient<NormalizedCacheObject>
  private readonly debug: Debugger.Debugger
  private readonly queryDebug: Debugger.Debugger
  private readonly tryDebug: Debugger.Debugger

  constructor(queryNodeProvider: ApolloClient<NormalizedCacheObject>) {
    this.queryNodeProvider = queryNodeProvider
    this.debug = Debugger('query-node-api')
    this.queryDebug = this.debug.extend('query')
    this.tryDebug = this.debug.extend('try')
  }

  public tryQueryWithTimeout<QueryResultT>(
    query: () => Promise<QueryResultT>,
    assertResultIsValid: (res: QueryResultT) => void,
    timeoutMs = 210000,
    retryTimeMs = 30000
  ): Promise<QueryResultT> {
    const label = query.toString().replace(/^.*\.([A-za-z0-9]+\(.*\))$/g, '$1')
    const retryDebug = this.tryDebug.extend(label).extend('retry')
    const failDebug = this.tryDebug.extend(label).extend('failed')
    return new Promise((resolve, reject) => {
      let lastError: any
      const timeout = setTimeout(() => {
        failDebug(`Query node query is still failing after timeout was reached (${timeoutMs}ms)!`)
        reject(lastError)
      }, timeoutMs)
      const tryQuery = () => {
        query()
          .then((result) => {
            try {
              assertResultIsValid(result)
              clearTimeout(timeout)
              resolve(result)
            } catch (e) {
              retryDebug(
                `Unexpected query result${
                  e && e.message ? ` (${e.message})` : ''
                }, retyring query in ${retryTimeMs}ms...`
              )
              lastError = e
              setTimeout(tryQuery, retryTimeMs)
            }
          })
          .catch((e) => {
            retryDebug(`Query node unreachable, retyring query in ${retryTimeMs}ms...`)
            lastError = e
            setTimeout(tryQuery, retryTimeMs)
          })
      }

      tryQuery()
    })
  }

  public async getMemberById(id: MemberId): Promise<ApolloQueryResult<Pick<Query, 'membershipByUniqueInput'>>> {
    const MEMBER_BY_ID_QUERY = gql`
      query($id: ID!) {
        membershipByUniqueInput(where: { id: $id }) {
          id
          handle
          metadata {
            name
            about
          }
          controllerAccount
          rootAccount
          registeredAtBlock {
            number
            timestamp
            network
          }
          registeredAtTime
          entry
          isVerified
          inviteCount
          invitedBy {
            id
          }
          invitees {
            id
          }
          boundAccounts
        }
      }
    `

    this.queryDebug(`Executing getMemberById(${id.toString()}) query`)

    return this.queryNodeProvider.query({ query: MEMBER_BY_ID_QUERY, variables: { id: id.toNumber() } })
  }

  public async getMembershipBoughtEvents(
    memberId: MemberId
  ): Promise<ApolloQueryResult<Pick<Query, 'membershipBoughtEvents'>>> {
    const MEMBERTSHIP_BOUGHT_BY_MEMBER_ID = gql`
      query($memberId: ID!) {
        membershipBoughtEvents(where: { newMemberId_eq: $memberId }) {
          ${EVENT_GENERIC_FIELDS}
          newMember {
            id
          }
          rootAccount
          controllerAccount
          handle
          metadata {
            name
            about
          }
          referrer {
            id
          }
        }
      }
    `

    this.queryDebug(`Executing getMembershipBoughtEvents(${memberId.toString()})`)

    return this.queryNodeProvider.query({
      query: MEMBERTSHIP_BOUGHT_BY_MEMBER_ID,
      variables: { memberId: memberId.toNumber() },
    })
  }

  public async getMemberProfileUpdatedEvents(
    memberId: MemberId
  ): Promise<ApolloQueryResult<Pick<Query, 'memberProfileUpdatedEvents'>>> {
    const MEMBER_PROFILE_UPDATED_BY_MEMBER_ID = gql`
      query($memberId: ID!) {
        memberProfileUpdatedEvents(where: { memberId_eq: $memberId }) {
          ${EVENT_GENERIC_FIELDS}
          member {
            id
          }
          newHandle
          newMetadata {
            name
            about
          }
        }
      }
    `

    this.queryDebug(`Executing getMemberProfileUpdatedEvents(${memberId.toString()})`)

    return this.queryNodeProvider.query({
      query: MEMBER_PROFILE_UPDATED_BY_MEMBER_ID,
      variables: { memberId: memberId.toNumber() },
    })
  }

  public async getMemberAccountsUpdatedEvents(
    memberId: MemberId
  ): Promise<ApolloQueryResult<Pick<Query, 'memberAccountsUpdatedEvents'>>> {
    const MEMBER_ACCOUNTS_UPDATED_BY_MEMBER_ID = gql`
      query($memberId: ID!) {
        memberAccountsUpdatedEvents(where: { memberId_eq: $memberId }) {
          ${EVENT_GENERIC_FIELDS}
          member {
            id
          }
          newRootAccount
          newControllerAccount
        }
      }
    `

    this.queryDebug(`Executing getMemberAccountsUpdatedEvents(${memberId.toString()})`)

    return this.queryNodeProvider.query({
      query: MEMBER_ACCOUNTS_UPDATED_BY_MEMBER_ID,
      variables: { memberId: memberId.toNumber() },
    })
  }

  public async getMemberInvitedEvents(
    memberId: MemberId
  ): Promise<ApolloQueryResult<Pick<Query, 'memberInvitedEvents'>>> {
    const MEMBER_INVITED_BY_MEMBER_ID = gql`
      query($memberId: ID!) {
        memberInvitedEvents(where: { newMemberId_eq: $memberId }) {
          ${EVENT_GENERIC_FIELDS}
          invitingMember {
            id
          }
          newMember {
            id
          }
          rootAccount
          controllerAccount
          handle
          metadata {
            name
            about
          }
        }
      }
    `

    this.queryDebug(`Executing getMemberInvitedEvents(${memberId.toString()})`)

    return this.queryNodeProvider.query({
      query: MEMBER_INVITED_BY_MEMBER_ID,
      variables: { memberId: memberId.toNumber() },
    })
  }

  public async getInvitesTransferredEvents(
    fromMemberId: MemberId
  ): Promise<ApolloQueryResult<Pick<Query, 'invitesTransferredEvents'>>> {
    const INVITES_TRANSFERRED_BY_MEMBER_ID = gql`
      query($from: ID!) {
        invitesTransferredEvents(where: { sourceMemberId_eq: $from }) {
          ${EVENT_GENERIC_FIELDS}
          sourceMember {
            id
          }
          targetMember {
            id
          }
          numberOfInvites
        }
      }
    `

    this.queryDebug(`Executing getInvitesTransferredEvents(${fromMemberId.toString()})`)

    return this.queryNodeProvider.query({
      query: INVITES_TRANSFERRED_BY_MEMBER_ID,
      variables: { from: fromMemberId.toNumber() },
    })
  }

  public async getStakingAccountAddedEvents(
    memberId: MemberId
  ): Promise<ApolloQueryResult<Pick<Query, 'stakingAccountAddedEvents'>>> {
    const STAKING_ACCOUNT_ADDED_BY_MEMBER_ID = gql`
      query($memberId: ID!) {
        stakingAccountAddedEvents(where: { memberId_eq: $memberId }) {
          ${EVENT_GENERIC_FIELDS}
          member {
            id
          }
          account
        }
      }
    `

    this.queryDebug(`Executing getStakingAccountAddedEvents(${memberId.toString()})`)

    return this.queryNodeProvider.query({
      query: STAKING_ACCOUNT_ADDED_BY_MEMBER_ID,
      variables: { memberId: memberId.toNumber() },
    })
  }

  public async getStakingAccountConfirmedEvents(
    memberId: MemberId
  ): Promise<ApolloQueryResult<Pick<Query, 'stakingAccountConfirmedEvents'>>> {
    const STAKING_ACCOUNT_CONFIRMED_BY_MEMBER_ID = gql`
      query($memberId: ID!) {
        stakingAccountConfirmedEvents(where: { memberId_eq: $memberId }) {
          ${EVENT_GENERIC_FIELDS}
          member {
            id
          }
          account
        }
      }
    `

    this.queryDebug(`Executing getStakingAccountConfirmedEvents(${memberId.toString()})`)

    return this.queryNodeProvider.query({
      query: STAKING_ACCOUNT_CONFIRMED_BY_MEMBER_ID,
      variables: { memberId: memberId.toNumber() },
    })
  }

  public async getStakingAccountRemovedEvents(
    memberId: MemberId
  ): Promise<ApolloQueryResult<Pick<Query, 'stakingAccountRemovedEvents'>>> {
    const STAKING_ACCOUNT_REMOVED_BY_MEMBER_ID = gql`
      query($memberId: ID!) {
        stakingAccountRemovedEvents(where: { memberId_eq: $memberId }) {
          ${EVENT_GENERIC_FIELDS}
          member {
            id
          }
          account
        }
      }
    `

    this.queryDebug(`Executing getStakingAccountRemovedEvents(${memberId.toString()})`)

    return this.queryNodeProvider.query({
      query: STAKING_ACCOUNT_REMOVED_BY_MEMBER_ID,
      variables: { memberId: memberId.toNumber() },
    })
  }

  // FIXME: Cross-filtering is not enabled yet, so we have to use timestamp workaround
  public async getMembershipSystemSnapshot(
    timestamp: number,
    matchType: 'eq' | 'lt' | 'lte' | 'gt' | 'gte' = 'eq'
  ): Promise<MembershipSystemSnapshot | undefined> {
    const MEMBERSHIP_SYSTEM_SNAPSHOT_QUERY = gql`
      query($time: DateTime!) {
        membershipSystemSnapshots(where: { snapshotTime_${matchType}: $time }, orderBy: snapshotTime_DESC, limit: 1) {
          snapshotBlock {
            timestamp
            network
            number
          }
          snapshotTime
          referralCut
          invitedInitialBalance
          defaultInviteCount
          membershipPrice
        }
      }
    `

    this.queryDebug(`Executing getMembershipSystemSnapshot(${matchType} ${timestamp})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'membershipSystemSnapshots'>>({
        query: MEMBERSHIP_SYSTEM_SNAPSHOT_QUERY,
        variables: { time: new Date(timestamp) },
      })
    ).data.membershipSystemSnapshots[0]
  }

  public async getReferralCutUpdatedEvent(
    blockNumber: number,
    indexInBlock: number
  ): Promise<ReferralCutUpdatedEvent | undefined> {
    const REFERRAL_CUT_UPDATED_BY_ID = gql`
      query($eventId: ID!) {
        referralCutUpdatedEvents(where: { eventId_eq: $eventId }) {
          ${EVENT_GENERIC_FIELDS}
          newValue
        }
      }
    `

    const eventId = `${blockNumber}-${indexInBlock}`
    this.queryDebug(`Executing getReferralCutUpdatedEvent(${eventId})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'referralCutUpdatedEvents'>>({
        query: REFERRAL_CUT_UPDATED_BY_ID,
        variables: { eventId },
      })
    ).data.referralCutUpdatedEvents[0]
  }

  public async getMembershipPriceUpdatedEvent(
    blockNumber: number,
    indexInBlock: number
  ): Promise<MembershipPriceUpdatedEvent | undefined> {
    const MEMBERSHIP_PRICE_UPDATED_BY_ID = gql`
      query($eventId: ID!) {
        membershipPriceUpdatedEvents(where: { eventId_eq: $eventId }) {
          ${EVENT_GENERIC_FIELDS}
          newPrice
        }
      }
    `

    const eventId = `${blockNumber}-${indexInBlock}`
    this.queryDebug(`Executing getMembershipPriceUpdatedEvent(${eventId})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'membershipPriceUpdatedEvents'>>({
        query: MEMBERSHIP_PRICE_UPDATED_BY_ID,
        variables: { eventId },
      })
    ).data.membershipPriceUpdatedEvents[0]
  }

  public async getInitialInvitationBalanceUpdatedEvent(
    blockNumber: number,
    indexInBlock: number
  ): Promise<InitialInvitationBalanceUpdatedEvent | undefined> {
    const INITIAL_INVITATION_BALANCE_UPDATED_BY_ID = gql`
      query($eventId: ID!) {
        initialInvitationBalanceUpdatedEvents(where: { eventId_eq: $eventId }) {
          ${EVENT_GENERIC_FIELDS}
          newInitialBalance
        }
      }
    `

    const eventId = `${blockNumber}-${indexInBlock}`
    this.queryDebug(`Executing getInitialInvitationBalanceUpdatedEvent(${eventId})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'initialInvitationBalanceUpdatedEvents'>>({
        query: INITIAL_INVITATION_BALANCE_UPDATED_BY_ID,
        variables: { eventId },
      })
    ).data.initialInvitationBalanceUpdatedEvents[0]
  }

  public async getInitialInvitationCountUpdatedEvent(
    blockNumber: number,
    indexInBlock: number
  ): Promise<InitialInvitationCountUpdatedEvent | undefined> {
    const INITIAL_INVITATION_COUNT_UPDATED_BY_ID = gql`
      query($eventId: ID!) {
        initialInvitationCountUpdatedEvents(where: { eventId_eq: $eventId }) {
          ${EVENT_GENERIC_FIELDS}
          newInitialInvitationCount
        }
      }
    `

    const eventId = `${blockNumber}-${indexInBlock}`
    this.queryDebug(`Executing getInitialInvitationCountUpdatedEvent(${eventId})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'initialInvitationCountUpdatedEvents'>>({
        query: INITIAL_INVITATION_COUNT_UPDATED_BY_ID,
        variables: { eventId },
      })
    ).data.initialInvitationCountUpdatedEvents[0]
  }

  public async getOpeningById(
    id: OpeningId,
    group: WorkingGroupModuleName
  ): Promise<ApolloQueryResult<Pick<Query, 'workingGroupOpeningByUniqueInput'>>> {
    const OPENING_BY_ID = gql`
      query($openingId: ID!) {
        workingGroupOpeningByUniqueInput(where: { id: $openingId }) {
          id
          runtimeId
          group {
            name
            leader {
              runtimeId
            }
          }
          applications {
            id
            runtimeId
            status {
              __typename
              ... on ApplicationStatusCancelled {
                openingCancelledEventId
              }
              ... on ApplicationStatusWithdrawn {
                applicationWithdrawnEventId
              }
              ... on ApplicationStatusAccepted {
                openingFilledEventId
              }
              ... on ApplicationStatusRejected {
                openingFilledEventId
              }
            }
          }
          type
          status {
            __typename
            ... on OpeningStatusFilled {
              openingFilledEventId
            }
            ... on OpeningStatusCancelled {
              openingCancelledEventId
            }
          }
          metadata {
            shortDescription
            description
            hiringLimit
            expectedEnding
            applicationDetails
            applicationFormQuestions {
              question
              type
              index
            }
          }
          stakeAmount
          unstakingPeriod
          rewardPerBlock
          createdAtBlock {
            number
            timestamp
            network
          }
          createdAt
        }
      }
    `

    const openingId = `${group}-${id.toString()}`
    this.queryDebug(`Executing getOpeningById(${openingId})`)

    return this.queryNodeProvider.query<Pick<Query, 'workingGroupOpeningByUniqueInput'>>({
      query: OPENING_BY_ID,
      variables: { openingId },
    })
  }

  public async getApplicationById(
    id: ApplicationId,
    group: WorkingGroupModuleName
  ): Promise<ApolloQueryResult<Pick<Query, 'workingGroupApplicationByUniqueInput'>>> {
    const APPLICATION_BY_ID = gql`
      query($applicationId: ID!) {
        workingGroupApplicationByUniqueInput(where: { id: $applicationId }) {
          id
          runtimeId
          createdAtBlock {
            number
            timestamp
            network
          }
          createdAt
          opening {
            id
            runtimeId
          }
          applicant {
            id
          }
          roleAccount
          rewardAccount
          stakingAccount
          status {
            __typename
            ... on ApplicationStatusCancelled {
              openingCancelledEventId
            }
            ... on ApplicationStatusWithdrawn {
              applicationWithdrawnEventId
            }
            ... on ApplicationStatusAccepted {
              openingFilledEventId
            }
            ... on ApplicationStatusRejected {
              openingFilledEventId
            }
          }
          answers {
            question {
              question
            }
            answer
          }
          stake
        }
      }
    `

    const applicationId = `${group}-${id.toString()}`
    this.queryDebug(`Executing getApplicationById(${applicationId})`)

    return this.queryNodeProvider.query<Pick<Query, 'workingGroupApplicationByUniqueInput'>>({
      query: APPLICATION_BY_ID,
      variables: { applicationId },
    })
  }

  public async getAppliedOnOpeningEvent(
    blockNumber: number,
    indexInBlock: number
  ): Promise<AppliedOnOpeningEvent | undefined> {
    const APPLIED_ON_OPENING_BY_ID = gql`
      query($eventId: ID!) {
        appliedOnOpeningEvents(where: { eventId_eq: $eventId }) {
          ${EVENT_GENERIC_FIELDS}
          group {
            name
          }
          opening {
            id
            runtimeId
          }
          application {
            id
            runtimeId
          }
        }
      }
    `

    const eventId = `${blockNumber}-${indexInBlock}`
    this.queryDebug(`Executing getAppliedOnOpeningEvent(${eventId})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'appliedOnOpeningEvents'>>({
        query: APPLIED_ON_OPENING_BY_ID,
        variables: { eventId },
      })
    ).data.appliedOnOpeningEvents[0]
  }

  public async getOpeningAddedEvent(blockNumber: number, indexInBlock: number): Promise<OpeningAddedEvent | undefined> {
    const OPENING_ADDED_BY_ID = gql`
      query($eventId: ID!) {
        openingAddedEvents(where: { eventId_eq: $eventId }) {
          ${EVENT_GENERIC_FIELDS}
          group {
            name
          }
          opening {
            id
            runtimeId
          }
        }
      }
    `

    const eventId = `${blockNumber}-${indexInBlock}`
    this.queryDebug(`Executing getOpeningAddedEvent(${eventId})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'openingAddedEvents'>>({
        query: OPENING_ADDED_BY_ID,
        variables: { eventId },
      })
    ).data.openingAddedEvents[0]
  }

  public async getOpeningFilledEvent(
    blockNumber: number,
    indexInBlock: number
  ): Promise<OpeningFilledEvent | undefined> {
    const OPENING_FILLED_BY_ID = gql`
      query($eventId: ID!) {
        openingFilledEvents(where: { eventId_eq: $eventId }) {
          ${EVENT_GENERIC_FIELDS}
          group {
            name
          }
          opening {
            id
            runtimeId
          }
          workersHired {
            id
            runtimeId
            group {
              name
            }
            membership {
              id
            }
            roleAccount
            rewardAccount
            stakeAccount
            status {
              __typename
            }
            isLead
            stake
            payouts {
              id
            }
            hiredAtBlock {
              number
              timestamp
              network
            }
            hiredAtTime
            application {
              id
              runtimeId
            }
            storage
          }
        }
      }
    `

    const eventId = `${blockNumber}-${indexInBlock}`
    this.queryDebug(`Executing getOpeningFilledEvent(${eventId})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'openingFilledEvents'>>({
        query: OPENING_FILLED_BY_ID,
        variables: { eventId },
      })
    ).data.openingFilledEvents[0]
  }

  public async getApplicationWithdrawnEvent(
    blockNumber: number,
    indexInBlock: number
  ): Promise<ApplicationWithdrawnEvent | undefined> {
    const APPLICATION_WITHDRAWN_BY_ID = gql`
      query($eventId: ID!) {
        applicationWithdrawnEvents(where: { eventId_eq: $eventId }) {
          ${EVENT_GENERIC_FIELDS}
          group {
            name
          }
          application {
            id
            runtimeId
          }
        }
      }
    `

    const eventId = `${blockNumber}-${indexInBlock}`
    this.queryDebug(`Executing getApplicationWithdrawnEvent(${eventId})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'applicationWithdrawnEvents'>>({
        query: APPLICATION_WITHDRAWN_BY_ID,
        variables: { eventId },
      })
    ).data.applicationWithdrawnEvents[0]
  }

  public async getOpeningCancelledEvent(
    blockNumber: number,
    indexInBlock: number
  ): Promise<OpeningCanceledEvent | undefined> {
    const OPENING_CANCELLED_BY_ID = gql`
      query($eventId: ID!) {
        openingCanceledEvents(where: { eventId_eq: $eventId }) {
          ${EVENT_GENERIC_FIELDS}
          group {
            name
          }
          opening {
            id
            runtimeId
          }
        }
      }
    `

    const eventId = `${blockNumber}-${indexInBlock}`
    this.queryDebug(`Executing getOpeningCancelledEvent(${eventId})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'openingCanceledEvents'>>({
        query: OPENING_CANCELLED_BY_ID,
        variables: { eventId },
      })
    ).data.openingCanceledEvents[0]
  }

  public async getStatusTextChangedEvent(
    blockNumber: number,
    indexInBlock: number
  ): Promise<StatusTextChangedEvent | undefined> {
    const STATUS_TEXT_CHANGED_BY_ID = gql`
      query($eventId: ID!) {
        statusTextChangedEvents(where: { eventId_eq: $eventId }) {
          ${EVENT_GENERIC_FIELDS}
          group {
            name
          }
          metadata
          result {
            ... on UpcomingOpeningAdded {
              upcomingOpeningId
            }
            ... on UpcomingOpeningRemoved {
              upcomingOpeningId
            }
            ... on WorkingGroupMetadataSet {
              metadataId
            }
            ... on InvalidActionMetadata {
              reason
            }
          }
        }
      }
    `

    const eventId = `${blockNumber}-${indexInBlock}`
    this.queryDebug(`Executing getStatusTextChangedEvent(${eventId})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'statusTextChangedEvents'>>({
        query: STATUS_TEXT_CHANGED_BY_ID,
        variables: { eventId },
      })
    ).data.statusTextChangedEvents[0]
  }

  public async getUpcomingOpeningByCreatedInEventId(eventId: string): Promise<UpcomingWorkingGroupOpening | undefined> {
    const UPCOMING_OPENING_BY_ID = gql`
      query($eventId: ID!) {
        upcomingWorkingGroupOpenings(where: { createdInEventId_eq: $eventId }) {
          id
          group {
            name
          }
          metadata {
            shortDescription
            description
            hiringLimit
            expectedEnding
            applicationDetails
            applicationFormQuestions {
              question
              type
              index
            }
          }
          expectedStart
          stakeAmount
          rewardPerBlock
          createdAtBlock {
            number
            timestamp
            network
          }
          createdAt
        }
      }
    `

    this.queryDebug(`Executing getUpcomingOpeningByCreatedInEventId(${eventId})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'upcomingWorkingGroupOpenings'>>({
        query: UPCOMING_OPENING_BY_ID,
        variables: { eventId },
      })
    ).data.upcomingWorkingGroupOpenings[0]
  }

  public async getWorkingGroup(name: WorkingGroupModuleName): Promise<WorkingGroup | undefined> {
    const GROUP_BY_NAME = gql`
      query($name: String!) {
        workingGroupByUniqueInput(where: { name: $name }) {
          name
          metadata {
            id
            status
            statusMessage
            about
            description
            setAtBlock {
              number
            }
          }
          leader {
            id
          }
          budget
        }
      }
    `

    this.queryDebug(`Executing getWorkingGroup(${name})`)

    return (
      (
        await this.queryNodeProvider.query<Pick<Query, 'workingGroupByUniqueInput'>>({
          query: GROUP_BY_NAME,
          variables: { name },
        })
      ).data.workingGroupByUniqueInput || undefined
    )
  }

  // FIXME: Use blockheights once possible
  public async getGroupMetaSnapshot(
    timestamp: number,
    matchType: 'eq' | 'lt' | 'lte' | 'gt' | 'gte' = 'eq'
  ): Promise<WorkingGroupMetadata | undefined> {
    const GROUP_META_SNAPSHOT_BY_TIMESTAMP = gql`
      query($timestamp: DateTime!) {
        workingGroupMetadata(where: { createdAt_${matchType}: $timestamp, createdAt_lte: $toTime }, orderBy: createdAt_DESC, limit: 1) {
          id
          status
          statusMessage
          about
          description
          setAtBlock {
            number
          }
        }
      }
    `

    this.queryDebug(`Executing getGroupMetaSnapshot(${timestamp}, ${matchType})`)

    return (
      await this.queryNodeProvider.query<Pick<Query, 'workingGroupMetadata'>>({
        query: GROUP_META_SNAPSHOT_BY_TIMESTAMP,
        variables: { timestamp: new Date(timestamp) },
      })
    ).data.workingGroupMetadata[0]
  }
}
