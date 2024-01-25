#![allow(clippy::unused_async)]
use axum::response::IntoResponse;
use loco_rs::prelude::*;

use crate::initializers::view_templates::{Engine, TemplateEngine, TeraView};

pub async fn render_home(tera: Engine<TeraView>) -> Result<impl IntoResponse> {
    let res = tera.render("home/hello.html", ()).expect("templ");
    format::html(&res)
    /*
    ergonomics: pushing out rendered content
    ========================================

    // sets headers
    // "rides" a known concept, pit of success
    // can be toggle on/off with feature flag
    //
    // concept conflict: what about the src/view library which is
    // used for API serde views? users will look for views in there

    format::view(e, home, data)

    // so maybe

    format::template(e, home, data)

    // which means we want to change assets/views to assets/templates
    // templates is not Railsy, but it is Rustic.
    // src/views can host view functions.

    /*

    deals only with taking a domain object, adapting it to a template,
    executing the template. the controller interface has no idea what
    mechanism is used to render output, nice and happy separation of concerns.

    home(e: eng, user: User) -> impl ...{
        format::template(e, "dashboard/home.html", json!(name: user.name))
    }

    and then in controller we:

    views::dashboard::home(e, current_user) // views is app-local, it is src/views

     */


    other options
    =============

    // Redux-esque
    // strong types a view...? maybe, maybe not
    // how to set header?
    // meh.
    view = create_view(home) // const, closes over string, activates engine
    view(e, data)

    // follow an Axum approach.
    // introduces a new type, no pit of success
    ViewResponse(tera, home, data) // impls intoresponse
    */
}

pub fn routes() -> Routes {
    Routes::new().prefix("views").add("/home", get(render_home))
}
