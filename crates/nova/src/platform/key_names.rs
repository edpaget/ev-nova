//! The key names port's adapters: none reads the player's layout yet, so
//! every platform answers `None` and every key reads its US legend. The
//! key-bindings roadmap's phase 5 swaps in the real adapters here.

use std::rc::Rc;

use nova_view::{KeyNames, NoKeyNames};

/// The running platform's key names: [`NoKeyNames`] everywhere, for now.
#[must_use]
pub fn system_key_names() -> Rc<dyn KeyNames> {
    Rc::new(NoKeyNames)
}

#[cfg(test)]
mod tests {
    use nova_view::key_names::fixture::check_contract;

    use super::*;

    #[test]
    fn the_platform_s_key_names_meet_the_port_s_contract() {
        assert_eq!(check_contract(&NoKeyNames), Ok(()));
        let names = system_key_names();
        assert_eq!(names.name(nova_view::Key::KeyW), None);
    }
}
