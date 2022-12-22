#[macro_use]
extern crate rocket;
use rocket::{
    form::Form,
    fs::{relative, FileServer, TempFile},
    response::Redirect,
    routes,
};
use sanitize_filename::sanitize;
use std::{
    fs::{set_permissions, Permissions},
    os::unix::prelude::PermissionsExt,
    time::{SystemTime, UNIX_EPOCH},
};

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/", FileServer::from(relative!("public")))
}

#[post("/", data = "<data>")]
async fn index(mut data: Form<Upload<'_>>) -> Result<Redirect, Redirect> {
    if data.files.len() == 1 && data.files[0].len() == 0 {
        return Err(Redirect::to(uri!(result("No files selected"))));
    }

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time traveling not supported")
        .as_millis();
    let privacy = data.privacy.clone();
    let mut errors = vec![];

    for (i, file) in data.files.iter_mut().enumerate() {
        if file.len() == 0 {
            errors.push(i);
            continue;
        }

        let filename = format!("{time}_{}", filename(file, i));

        let result = if privacy == Privacy::Public {
            file.move_copy_to(format!("uploads/public/{filename}"))
                .await
        } else {
            file.move_copy_to(format!("uploads/private/{filename}"))
                .await
        };

        if result.is_err() {
            errors.push(i);
        } else {
            _ = set_permissions(file.path().unwrap(), Permissions::from_mode(0o644));
        }
    }
    if errors.is_empty() {
        Ok(Redirect::to(uri!("/result")))
    } else {
        let error = format!(
            "Error uploading {} files with the following indexes {errors:?}",
            errors.len()
        );
        Err(Redirect::to(uri!(result(error))))
    }
}

fn filename(file: &mut TempFile, i: usize) -> String {
    if let Some(filename) = file.raw_name() {
        return sanitize(filename.dangerous_unsafe_unsanitized_raw());
    }
    format!("{i}")
}

#[get("/result?<error>")]
#[allow(unused)]
fn result(error: &str) {}

#[derive(FromForm)]
struct Upload<'r> {
    files: Vec<TempFile<'r>>,
    #[field(name = "data-usage-agreement-radio")]
    privacy: Privacy,
}

#[derive(Clone, PartialEq, FromFormField)]
enum Privacy {
    #[field(value = "can-be-public")]
    Public,
    #[field(value = "keep-private")]
    Private,
}
