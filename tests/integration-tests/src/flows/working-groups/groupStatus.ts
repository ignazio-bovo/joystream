import { FlowProps } from '../../Flow'
import { UpdateGroupStatusFixture } from '../../fixtures/workingGroupsModule'

import Debugger from 'debug'
import { FixtureRunner } from '../../Fixture'
import { workingGroups } from '../../types'
import { WorkingGroupMetadata } from '@joystream/metadata-protobuf'
import _ from 'lodash'

export default async function upcomingOpenings({ api, query, env }: FlowProps): Promise<void> {
  await Promise.all(
    workingGroups.map(async (group) => {
      const updates: WorkingGroupMetadata.AsObject[] = [
        { description: `${_.startCase(group)} Test Description`, about: `${_.startCase(group)} Test About Text` },
        {
          status: 'Testing',
          statusMessage: `${_.startCase(group)} is beeing tested`,
        },
        {
          description: `${_.startCase(group)} New Test Description`,
        },
        {
          status: 'Testing continues',
          statusMessage: `${_.startCase(group)} testing continues`,
        },
        {
          about: `${_.startCase(group)} New Test About`,
        },
        {},
        {
          status: 'Testing finished',
          statusMessage: '',
          description: `${_.startCase(group)} Test Description`,
          about: `${_.startCase(group)} Test About Text`,
        },
      ]

      const debug = Debugger(`flow:group-status:${group}`)
      debug('Started')
      api.enableDebugTxLogs()

      // Run fixtures one-by-one (otherwise the checks may break)
      // FIXME
      for (const update of updates) {
        const updateGroupStatusFixture = new UpdateGroupStatusFixture(api, query, group, update)
        await new FixtureRunner(updateGroupStatusFixture).runWithQueryNodeChecks()
      }

      debug('Done')
    })
  )
}
