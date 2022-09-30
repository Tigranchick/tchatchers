use crate::components::common::FormButton;
use crate::router::Route;
use web_sys::HtmlInputElement;
use yew::{html, Component, Context, Html, NodeRef};
use yew_router::history::History;
use yew_router::scope_ext::RouterScopeExt;

pub enum Msg {
    SubmitForm,
}

#[derive(Default)]
pub struct JoinRoom {
    room_name: NodeRef,
}

impl Component for JoinRoom {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self::default()
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SubmitForm => {
                if let Some(room_name) = self.room_name.cast::<HtmlInputElement>() {
                    if room_name.check_validity() {
                        ctx.link().history().unwrap().push(Route::Room {
                            room: room_name.value(),
                        });
                    }
                }
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                <div class="flex items-center justify-center h-full">
                <form class="w-full max-w-sm border-2 px-6 py-6  lg:py-14" onsubmit={ctx.link().callback(|_| Msg::SubmitForm)} action="javascript:void(0);">

                <h2 class="text-xl mb-10 text-center text-gray-500 font-bold">{"Join a room"}</h2>
                  <div class="md:flex md:items-center mb-6">
                    <div class="md:w-1/3">
                      <label class="block text-gray-500 font-bold md:text-right mb-1 md:mb-0 pr-4" for="inline-full-name">
                      {"Room name"}
                      </label>
                    </div>
                    <div class="md:w-2/3">
                      <input class="peer bg-gray-200 appearance-none border-2 border-gray-200 rounded w-full py-2 px-4 text-gray-700 leading-tight focus:outline-none focus:bg-white focus:border-purple-500 focus:invalid:border-red-500 visited:invalid:border-red-500" id="inline-full-name" type="text" required=true minlength="1" ref={&self.room_name} />
                    </div>
                  </div>
                  <FormButton label="Join room" />
                </form>
                </div>
            </>
        }
    }
}