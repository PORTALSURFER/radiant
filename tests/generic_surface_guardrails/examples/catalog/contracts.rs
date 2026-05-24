#[path = "contracts/advanced.rs"]
mod advanced;
#[path = "contracts/application.rs"]
mod application;
#[path = "contracts/first_use.rs"]
mod first_use;
#[path = "contracts/systems.rs"]
mod systems;

pub(super) type ExampleContract = (&'static str, &'static [&'static str]);

pub(super) const SEPARATELY_COVERED_EXAMPLES: &[&str] = &["generic_native", "hello_world"];

const FOCUSED_EXAMPLE_CONTRACT_GROUPS: &[&[ExampleContract]] = &[
    first_use::CONTRACTS,
    application::CONTRACTS,
    advanced::CONTRACTS,
    systems::CONTRACTS,
];

pub(super) fn focused_example_contracts() -> impl Iterator<Item = ExampleContract> {
    FOCUSED_EXAMPLE_CONTRACT_GROUPS
        .iter()
        .flat_map(|contracts| contracts.iter().copied())
}

pub(super) fn has_focused_example_contract(name: &str) -> bool {
    focused_example_contracts().any(|(contract_name, _)| contract_name == name)
}
