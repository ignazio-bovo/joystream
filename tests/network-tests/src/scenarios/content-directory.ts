import leaderSetup from '../flows/content/setupPreMigrationContent'
import { scenario } from '../Scenario'

scenario(async ({ job }) => {
  job('setup pre migration content', setupPreMigrationContent)
})

