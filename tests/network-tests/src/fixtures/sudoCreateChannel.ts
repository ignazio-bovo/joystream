import { BaseFixture } from '../Fixture'
import { BuyMembershipHappyCaseFixture } from './membershipModule'
import { Api } from '../Api'
import { OpeningId } from '@joystream/types/hiring'
import { PaidTermId } from '@joystream/types/members'
import { createTypeFromConstructor } from '@joystream/sumer-types'
import { ChannelCreationParameters } from '@joystream/types/content'


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

	// create channel as sudo account (Alice)
	this.api.sudoCreateChannel(channelCreationParameters)

	// assert number of outstanding channels > 0
    }
}
