use node_runtime::ProjectTokenConfig;

pub fn testing_config() -> ProjectTokenConfig {
    ProjectTokenConfig {
        account_info_by_token_and_account: vec![],
        token_info_by_id: vec![],
        next_token_id: 1,
        bloat_bond: 0,
        symbol_used: vec![],
    }
}

pub fn production_config() -> ProjectTokenConfig {
    ProjectTokenConfig {
        account_info_by_token_and_account: vec![],
        token_info_by_id: vec![],
        next_token_id: 1,
        bloat_bond: 10, // TODO(Ephesus): update bloat bond
        symbol_used: vec![],
    }
}
