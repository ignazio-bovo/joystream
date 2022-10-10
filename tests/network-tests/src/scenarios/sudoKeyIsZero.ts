import zeroSudoKeyDisablesSudo from '../flows/sudo/zeroSudoKeyDisablesSudo'
import { scenario } from '../Scenario'

// eslint-disable-next-line @typescript-eslint/no-floating-promises
scenario('Zero Sudo key disable sudo', async ({ job }) => {
  job('setting sudo key to zero disables sudo pallet', zeroSudoKeyDisablesSudo)
})
