pub mod components;
pub mod router;
pub mod services;

use components::navbar::Navbar;
use router::{switch, Route};
use yew::prelude::*;
use yew_router::prelude::*;
use crate::components::auth_checker::AuthChecker;

#[macro_use]
extern crate derive_more;

#[function_component(App)]
fn app() -> Html {
    html! {
        <>
            <BrowserRouter>
                <div class="h-screen grid grid-rows-12">
                    <Navbar />
                    <div class="row-span-11">
                        <Switch<Route> render={Switch::render(switch)} />
                    </div>
                </div>
            </BrowserRouter>
            <AuthChecker/>
        </>
    }
}

fn main() {
    yew::start_app::<App>();
}
