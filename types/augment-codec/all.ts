// This file was automatically generated via generate:augment-codec
import { Credential, CredentialSet, BlockAndTime, ThreadId, PostId, InputValidationLengthConstraint, WorkingGroup, SlashingTerms, SlashableTerms, MemoText, Address, LookupSource } from '../common';
import { EntryMethod, MemberId, PaidTermId, SubscriptionId, Membership, PaidMembershipTerms, ActorId } from '../members';
import { ElectionStage, ElectionStake, SealedVote, TransferableStake, ElectionParameters, Seat, Seats, Backer, Backers } from '../council';
import { RoleParameters } from '../roles';
import { PostTextChange, ModerationAction, ChildPositionInParentCategory, CategoryId, Category, Thread, Post, ReplyId, Reply } from '../forum';
import { StakeId, Stake, StakingStatus, Staked, StakedStatus, Unstaking, Slash } from '../stake';
import { MintId, Mint, MintBalanceOf, BalanceOfMint, NextAdjustment, AdjustOnInterval, AdjustCapacityBy } from '../mint';
import { RecipientId, RewardRelationshipId, Recipient, RewardRelationship } from '../recurring-rewards';
import { ApplicationId, OpeningId, Application, ApplicationStage, ActivateOpeningAt, ApplicationRationingPolicy, OpeningStage, StakingPolicy, Opening, WaitingToBeingOpeningStageVariant, ActiveOpeningStageVariant, ActiveOpeningStage, AcceptingApplications, ReviewPeriod, Deactivated, OpeningDeactivationCause, InactiveApplicationStage, UnstakingApplicationStage, ApplicationDeactivationCause, StakingAmountLimitMode } from '../hiring';
import { ChannelId, CuratorId, CuratorOpeningId, CuratorApplicationId, LeadId, PrincipalId, OptionalText, Channel, ChannelContentType, ChannelCurationStatus, ChannelPublicationStatus, CurationActor, Curator, CuratorApplication, CuratorOpening, Lead, OpeningPolicyCommitment, Principal, WorkingGroupUnstaker, CuratorApplicationIdToCuratorIdMap, CuratorApplicationIdSet, CuratorRoleStakeProfile, CuratorRoleStage, CuratorExitSummary, CuratorExitInitiationOrigin, LeadRoleState, ExitedLeadRole, CuratorInduction } from '../content-working-group';
import { RationaleText, Application as ApplicationOf, ApplicationIdSet, ApplicationIdToWorkerIdMap, WorkerId, Worker as WorkerOf, Opening as OpeningOf, StorageProviderId, OpeningType, ApplicationId as HiringApplicationId, RewardPolicy, WorkingGroupOpeningPolicyCommitment, RoleStakeProfile } from '../working-group';
import { Url, IPNSIdentity, ServiceProviderRecord } from '../discovery';
import { ContentId, LiaisonJudgement, DataObject, DataObjectStorageRelationshipId, DataObjectStorageRelationship, DataObjectTypeId, DataObjectType, DataObjectsMap } from '../media';
import { ProposalId, ProposalStatus, Proposal as ProposalOf, ProposalDetails, ProposalDetails as ProposalDetailsOf, VotingResults, ProposalParameters, VoteKind, ThreadCounter, DiscussionThread, DiscussionPost, AddOpeningParameters, FillOpeningParameters, TerminateRoleParameters, ActiveStake, Finalized, ProposalDecisionStatus, ExecutionFailed, Approved, SetLeadParams } from '../proposals';
import { Nonce, EntityId, ClassId, CuratorGroupId, VecMaxLength, TextMaxLength, HashedTextMaxLength, PropertyId, SchemaId, SameController, ClassPermissions, PropertyTypeSingle, PropertyTypeVector, PropertyType, PropertyLockingPolicy, Property, Schema, Class, Class as ClassOf, EntityController, EntityPermissions, StoredValue, VecStoredValue, VecStoredPropertyValue, StoredPropertyValue, InboundReferenceCounter, Entity, Entity as EntityOf, CuratorGroup, EntityCreationVoucher, Actor, EntityReferenceCounterSideEffect, ReferenceCounterSideEffects, SideEffects, SideEffect, Status, InputValue, VecInputValue, InputPropertyValue, ParameterizedEntity, ParametrizedPropertyValue, ParametrizedClassPropertyValue, CreateEntityOperation, UpdatePropertyValuesOperation, AddSchemaSupportToEntityOperation, OperationType, InputEntityValuesMap, ClassPermissionsType, ClassPropertyValue, Operation, ReferenceConstraint, FailedAt } from '../content-directory';
import { StorageBucketId, StorageBucketsPerBagValueConstraint, DataObjectId, DynamicBagIdType, DynamicBagId, Voucher, DynamicBagType, DynamicBagCreationPolicy, DynamicBag, StaticBag, StorageBucket, StaticBagId, Static, BagId, DataObjectCreationParameters, BagIdType, UploadParameters, StorageBucketIdSet, DataObjectIdSet, ContentIdSet, ContentId } from '../storage';

export { Credential, CredentialSet, BlockAndTime, ThreadId, PostId, InputValidationLengthConstraint, WorkingGroup, SlashingTerms, SlashableTerms, MemoText, Address, LookupSource, EntryMethod, MemberId, PaidTermId, SubscriptionId, Membership, PaidMembershipTerms, ActorId, ElectionStage, ElectionStake, SealedVote, TransferableStake, ElectionParameters, Seat, Seats, Backer, Backers, RoleParameters, PostTextChange, ModerationAction, ChildPositionInParentCategory, CategoryId, Category, Thread, Post, ReplyId, Reply, StakeId, Stake, StakingStatus, Staked, StakedStatus, Unstaking, Slash, MintId, Mint, MintBalanceOf, BalanceOfMint, NextAdjustment, AdjustOnInterval, AdjustCapacityBy, RecipientId, RewardRelationshipId, Recipient, RewardRelationship, ApplicationId, OpeningId, Application, ApplicationStage, ActivateOpeningAt, ApplicationRationingPolicy, OpeningStage, StakingPolicy, Opening, WaitingToBeingOpeningStageVariant, ActiveOpeningStageVariant, ActiveOpeningStage, AcceptingApplications, ReviewPeriod, Deactivated, OpeningDeactivationCause, InactiveApplicationStage, UnstakingApplicationStage, ApplicationDeactivationCause, StakingAmountLimitMode, ChannelId, CuratorId, CuratorOpeningId, CuratorApplicationId, LeadId, PrincipalId, OptionalText, Channel, ChannelContentType, ChannelCurationStatus, ChannelPublicationStatus, CurationActor, Curator, CuratorApplication, CuratorOpening, Lead, OpeningPolicyCommitment, Principal, WorkingGroupUnstaker, CuratorApplicationIdToCuratorIdMap, CuratorApplicationIdSet, CuratorRoleStakeProfile, CuratorRoleStage, CuratorExitSummary, CuratorExitInitiationOrigin, LeadRoleState, ExitedLeadRole, CuratorInduction, RationaleText, ApplicationOf, ApplicationIdSet, ApplicationIdToWorkerIdMap, WorkerId, WorkerOf, OpeningOf, StorageProviderId, OpeningType, HiringApplicationId, RewardPolicy, WorkingGroupOpeningPolicyCommitment, RoleStakeProfile, Url, IPNSIdentity, ServiceProviderRecord, ContentId, LiaisonJudgement, DataObject, DataObjectStorageRelationshipId, DataObjectStorageRelationship, DataObjectTypeId, DataObjectType, DataObjectsMap, ProposalId, ProposalStatus, ProposalOf, ProposalDetails, ProposalDetailsOf, VotingResults, ProposalParameters, VoteKind, ThreadCounter, DiscussionThread, DiscussionPost, AddOpeningParameters, FillOpeningParameters, TerminateRoleParameters, ActiveStake, Finalized, ProposalDecisionStatus, ExecutionFailed, Approved, SetLeadParams, Nonce, EntityId, ClassId, CuratorGroupId, VecMaxLength, TextMaxLength, HashedTextMaxLength, PropertyId, SchemaId, SameController, ClassPermissions, PropertyTypeSingle, PropertyTypeVector, PropertyType, PropertyLockingPolicy, Property, Schema, Class, ClassOf, EntityController, EntityPermissions, StoredValue, VecStoredValue, VecStoredPropertyValue, StoredPropertyValue, InboundReferenceCounter, Entity, EntityOf, CuratorGroup, EntityCreationVoucher, Actor, EntityReferenceCounterSideEffect, ReferenceCounterSideEffects, SideEffects, SideEffect, Status, InputValue, VecInputValue, InputPropertyValue, ParameterizedEntity, ParametrizedPropertyValue, ParametrizedClassPropertyValue, CreateEntityOperation, UpdatePropertyValuesOperation, AddSchemaSupportToEntityOperation, OperationType, InputEntityValuesMap, ClassPermissionsType, ClassPropertyValue, Operation, ReferenceConstraint, FailedAt, StorageBucketId, StorageBucketsPerBagValueConstraint, DataObjectId, DynamicBagIdType, DynamicBagId, Voucher, DynamicBagType, DynamicBagCreationPolicy, DynamicBag, StaticBag, StorageBucket, StaticBagId, Static, BagId, DataObjectCreationParameters, BagIdType, UploadParameters, StorageBucketIdSet, DataObjectIdSet, ContentIdSet, ContentId };