use std::rc::Rc;

// Copyright ⓒ 2022 LABEYE Loïc
// This tool is distributed under the MIT License, check out [here](https://github.com/nag763/tchatchers/blob/main/LICENSE.MD).
use crate::components::common::AppButton;
use crate::components::common::FileAttacher;
use crate::components::common::WaitingForResponse;
use crate::components::toast::Alert;
use crate::router::Route;
use crate::services::modal_bus::ModalBus;
use crate::services::modal_bus::ModalBusContent;
use crate::services::toast_bus::ToastBus;
use crate::utils::jwt::get_user;
use crate::utils::requester::Requester;
use gloo_net::http::Request;
use tchatchers_core::user::PartialUser;
use tchatchers_core::user::UpdatableUser;
use tchatchers_core::validation_error_message::ValidationErrorMessage;
use validator::Validate;
use web_sys::HtmlInputElement;
use yew::{html, Callback, Component, Context, Html, NodeRef, Properties};
use yew_agent::Bridge;
use yew_agent::Bridged;
use yew_agent::Dispatched;
use yew_router::scope_ext::RouterScopeExt;

use super::modal::ModalContent;

pub enum Msg {
    UploadNewPfp(Option<js_sys::ArrayBuffer>),
    PfpUpdated(String),
    SubmitForm,
    ErrorFromServer(String),
    ProfileUpdated,
    FetchJwt,
    ConfirmDeletion,
    DeletionConfirmed,
}

#[derive(Clone, PartialEq, Eq, Properties)]
pub struct Props;

pub struct Settings {
    user: PartialUser,
    name: NodeRef,
    pfp: Option<String>,
    wait_for_api: bool,
    server_error: Option<String>,
    ok_msg: Option<String>,
    producer: Box<dyn Bridge<ModalBus>>,
}

impl Component for Settings {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::FetchJwt);
        let cb = {
            let link = ctx.link().clone();
            move |mc| {
                if let ModalBusContent::Outcome(true) = mc {
                    link.send_message(Msg::DeletionConfirmed)
                }
            }
        };
        Self {
            user: PartialUser::default(),
            name: NodeRef::default(),
            pfp: None,
            wait_for_api: false,
            server_error: None,
            ok_msg: None,
            producer: ModalBus::bridge(Rc::new(cb)),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::FetchJwt => {
                let user = match get_user() {
                    Ok(v) => v,
                    Err(e) => {
                        gloo_console::log!("Error while attempting to get the user :", e);
                        ctx.link().navigator().unwrap().push(&Route::SignIn);
                        ToastBus::dispatcher().send(Alert {
                            is_success: false,
                            content:
                                "Please authenticate prior accessing the app functionnalities."
                                    .into(),
                        });
                        PartialUser::default()
                    }
                };
                self.user = user.clone();
                self.pfp = user.pfp;
                true
            }
            Msg::SubmitForm => {
                self.wait_for_api = true;
                self.ok_msg = None;
                self.server_error = None;
                if let Some(name) = self.name.cast::<HtmlInputElement>() {
                    if name.check_validity() {
                        let payload = UpdatableUser {
                            id: self.user.id,
                            name: name.value(),
                            pfp: self.pfp.clone(),
                        };
                        if let Err(e) = payload.validate() {
                            let message: ValidationErrorMessage = e.into();
                            ctx.link()
                                .send_message(Msg::ErrorFromServer(message.to_string()));
                        } else {
                            let mut req = Requester::<UpdatableUser>::put("/api/user");
                            req.is_json(true).body(Some(payload));
                            let link = ctx.link().clone();
                            self.wait_for_api = true;
                            wasm_bindgen_futures::spawn_local(async move {
                                let resp = req.send().await;
                                if resp.status().is_success() {
                                    link.send_message(Msg::ProfileUpdated);
                                    ToastBus::dispatcher().send(Alert {
                                        is_success: true,
                                        content: "Your profile has been updated with success"
                                            .into(),
                                    });
                                } else {
                                    link.send_message(Msg::ErrorFromServer(
                                        resp.text().await.unwrap(),
                                    ));
                                }
                            });
                        }
                    }
                }
                true
            }
            Msg::UploadNewPfp(pfp) => {
                self.wait_for_api = true;
                let req = Request::post("/api/pfp").body(pfp.unwrap()).send();
                let link = ctx.link().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let resp = req.await.unwrap();
                    if resp.ok() {
                        link.send_message(Msg::PfpUpdated(resp.text().await.unwrap()));
                    } else {
                        link.send_message(Msg::ErrorFromServer(resp.text().await.unwrap()));
                    }
                });
                true
            }
            Msg::ErrorFromServer(s) => {
                self.wait_for_api = false;
                self.ok_msg = None;
                self.server_error = Some(s);
                true
            }
            Msg::PfpUpdated(pfp_path) => {
                self.wait_for_api = false;
                self.pfp = Some(pfp_path);
                true
            }
            Msg::ProfileUpdated => {
                self.wait_for_api = false;
                self.ok_msg = Some("Your profile has been updated with success.".into());
                ctx.link().send_message(Msg::FetchJwt);
                true
            }
            Msg::ConfirmDeletion => {
                let mc : ModalContent = ModalContent {
                    title: "You are about to delete your account".into(),
                    msg: "This action is not reversible, once your account is deleted, there is no way for you to get it back.".into(),
                    decline_text: Some("I changed, my mind, don't delete my account".into()),
                    accept_text: Some("Understood, farewell".into()),
                };
                self.producer.send(ModalBusContent::PopModal(mc));
                false
            }
            Msg::DeletionConfirmed => {
                let req = Requester::<()>::delete("/api/user");
                let link = ctx.link().clone();
                self.wait_for_api = true;
                wasm_bindgen_futures::spawn_local(async move {
                    let resp = req.send().await;
                    if resp.status().is_success() {
                        link.navigator().unwrap().push(&Route::LogOut);
                    } else {
                        link.send_message(Msg::ErrorFromServer(resp.text().await.unwrap()));
                    }
                });
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let pfp = match &self.user.pfp {
            None => match &self.pfp {
                Some(_) => {
                    html! {<span class="dark:text-gray-300">{"Your new profile picture is ready to be uploaded"}</span>}
                }
                None => {
                    html! { <span class="dark:text-gray-300">{"You don't have any profile picture so far"}</span> }
                }
            },
            Some(v) => html! { <><img class="h-10 w-10 rounded-full" src={v.clone()} /></> },
        };
        let link = ctx.link().clone();
        let end_of_form = match self.wait_for_api {
            true => html! { <WaitingForResponse /> },
            false => html! { <AppButton label="Update" /> },
        };
        let delete_profile = match self.wait_for_api {
            true => html! { <WaitingForResponse /> },
            false => {
                html! { <AppButton label="Delete" is_modal_opener=true callback={Callback::from(move |_ :()| {link.send_message(Msg::ConfirmDeletion)})}/> }
            }
        };
        let link = ctx.link().clone();
        html! {
            <>
                <div class="flex items-center justify-center h-full dark:bg-zinc-800">
                <form class="w-full max-w-sm border-2 dark:border-zinc-700 px-6 py-6  lg:py-14" onsubmit={ctx.link().callback(|_| Msg::SubmitForm)} action="javascript:void(0);" >

                <h2 class="text-xl mb-10 text-center text-gray-500 dark:text-gray-200 font-bold">{"Settings"}</h2>
                  <div class="md:flex md:items-center mb-6">
                    <div class="md:w-1/3">
                      <label class="block text-gray-500 dark:text-gray-200 font-bold md:text-right mb-1 md:mb-0 pr-4" for="inline-full-name">
                      {"Your login"}
                      </label>
                    </div>
                    <div class="md:w-2/3">
                      <input class="peer bg-gray-200 dark:bg-zinc-800 appearance-none border-2 border-gray-200 dark:border-zinc-700 rounded w-full py-2 px-4 text-gray-700 dark:text-gray-200 leading-tight focus:outline-none focus:bg-white dark:focus:bg-zinc-800 focus:border-zinc-500 focus:invalid:border-red-500 visited:invalid:border-red-500" id="inline-full-name" type="text" required=true minlength="3" maxlength="32" value={self.user.login.clone()} disabled=true/>
                    </div>
                    </div>
                  <div class="md:flex md:items-center mb-6">
                    <div class="md:w-1/3">
                      <label class="block text-gray-500 dark:text-gray-200 font-bold md:text-right mb-1 md:mb-0 pr-4" for="inline-full-name">
                      {"Your name"}
                      </label>
                    </div>
                    <div class="md:w-2/3">
                      <input class="peer bg-gray-200 dark:bg-zinc-800 appearance-none border-2 border-gray-200 dark:border-zinc-700 rounded w-full py-2 px-4 text-gray-700 dark:text-gray-200 leading-tight focus:outline-none focus:bg-white dark:focus:bg-zinc-800 focus:border-zinc-500 focus:invalid:border-red-500 visited:invalid:border-red-500" id="inline-full-name" type="text" required=true minlength="3" maxlength="16" ref={&self.name} value={self.user.name.clone()}/>
                    </div>
                  </div>
                  <div class="md:flex md:items-center mb-6">
                    <div class="md:w-1/3">
                      <label class="block text-gray-500 dark:text-gray-200 font-bold md:text-right mb-1 md:mb-0 pr-4" for="inline-full-name">
                      {"Your profile picture"}
                      </label>
                    </div>
                    <div class="md:w-2/3 flex justify-center items-center space-x-4 mt-2 dark:text-gray-200">
                    {pfp}
                    <FileAttacher disabled=false accept={Some(String::from(".png,.webp,.jpg,.jpg"))} on_file_attached={Callback::from(move |file_path: Option<js_sys::ArrayBuffer>| {
                        link.send_message(Msg::UploadNewPfp (file_path));
        })}/>
                    </div>
                  </div>
                  <small class="flex mt-4 mb-2 items-center text-red-500" hidden={self.server_error.is_none()}>
                    {self.server_error.as_ref().unwrap_or(&String::new())}
                  </small>
                  <small class="flex mt-4 mb-2 items-center text-green-500" hidden={self.ok_msg.is_none()}>
                    {self.ok_msg.as_ref().unwrap_or(&String::new())}
                  </small>
                  <div class="flex items-center">
                  <div class="w-1/3"></div>
                  <div class="flex flex-row w-2/3 justify-end space-x-3">
                     {delete_profile}
                     {end_of_form}
                  </div>
                </div>
                </form>
                </div>
            </>
        }
    }
}
