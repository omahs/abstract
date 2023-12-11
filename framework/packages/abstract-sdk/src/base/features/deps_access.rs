use cosmwasm_std::{Api, Deps, DepsMut, Env, MessageInfo};

pub trait DepsAccess {
    fn deps_mut<'a: 'b, 'b>(&'a mut self) -> DepsMut<'b>;
    fn deps<'a: 'b, 'b>(&'a self) -> Deps<'b>;

    fn env(&self) -> Env;
    fn message_info(&self) -> MessageInfo;

    fn api<'a: 'b, 'b>(&'a self) -> &'b dyn Api {
        self.deps().api
    }
}

impl DepsAccess for (DepsMut<'_>, Env, MessageInfo) {
    fn deps_mut<'a: 'b, 'b>(&'a mut self) -> cosmwasm_std::DepsMut<'b> {
        self.0.branch()
    }

    fn deps<'a: 'b, 'b>(&'a self) -> cosmwasm_std::Deps<'b> {
        self.0.as_ref()
    }

    fn env(&self) -> Env {
        self.1.clone()
    }

    fn message_info(&self) -> MessageInfo {
        self.2.clone()
    }
}

impl DepsAccess for (DepsMut<'_>, Env) {
    fn deps_mut<'a: 'b, 'b>(&'a mut self) -> cosmwasm_std::DepsMut<'b> {
        self.0.branch()
    }

    fn deps<'a: 'b, 'b>(&'a self) -> cosmwasm_std::Deps<'b> {
        self.0.as_ref()
    }

    fn env(&self) -> Env {
        self.1.clone()
    }

    fn message_info(&self) -> MessageInfo {
        unimplemented!()
    }
}

impl DepsAccess for (Deps<'_>, Env) {
    fn deps_mut<'a: 'b, 'b>(&'a mut self) -> cosmwasm_std::DepsMut<'b> {
        unimplemented!()
    }

    fn deps<'a: 'b, 'b>(&'a self) -> cosmwasm_std::Deps<'b> {
        self.0
    }

    fn env(&self) -> Env {
        self.1.clone()
    }

    fn message_info(&self) -> MessageInfo {
        unimplemented!()
    }
}
