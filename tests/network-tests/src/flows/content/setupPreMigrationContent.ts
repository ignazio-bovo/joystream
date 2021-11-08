import { Api } from '../../Api'
import { FlowProps } from '../../Flow'
import BN from 'bn.js'
import { SudoCreateContent } from '../../fixtures/sudoCreateChannel'
import { assert } from 'chai'
// import { KeyringPair } from '@polkadot/keyring/types'
import { FixtureRunner } from '../../Fixture'
import { extendDebug } from '../../Debugger'

// Worker application happy case scenario
async function setupPreMigrationContent(api: Api, env: NodeJS.ProcessEnv): Promise<void> {
    const debug = extendDebug(`flow:setupPreMigrationContent`)
    debug('Started')

    const leaderHiringHappyCaseFixture = new SudoCreateContent(api)
    await new FixtureRunner(leaderHiringHappyCaseFixture).run()

    debug('Done')

    // Who ever needs it will need to get it from the Api layer
    // return leadKeyPair
}
