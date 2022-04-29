use crate::{Balance, BlockNumber};
use sp_runtime::traits::Convert;

pub struct BlockNumberToBalance {}

impl Convert<BlockNumber, Balance> for BlockNumberToBalance {
    fn convert(block: BlockNumber) -> Balance {
        block as Balance
    }
}
