#[macro_use]
extern crate rocket;
use rocket::fs::{relative, FileServer, TempFile};
use rocket::{form::Form, response::Redirect, routes};
use sanitize_filename::sanitize;
use std::fs::{set_permissions, Permissions};
use std::os::unix::prelude::PermissionsExt;
use std::time::{SystemTime, UNIX_EPOCH};

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/", FileServer::from(relative!("public")))
}

#[post("/", data = "<data>")]
async fn index(mut data: Form<Upload<'_>>) -> Redirect {
    if data.files.len() == 1 && data.files[0].len() == 0 {
        return Redirect::to(uri!(error("No files selected")));
    }

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time traveling not supported")
        .as_millis();
    let privacy = data.privacy;
    let mut errors = vec![];

    for (i, file) in data.files.iter_mut().enumerate() {
        if file.len() == 0 {
            errors.push(i);
            continue;
        }

        let path = match privacy {
            Privacy::Public => format!("uploads/public/{time}_{}", filename(file, i)),
            Privacy::Private => format!("uploads/private/{time}_{}", filename(file, i)),
        };
        let result = file.move_copy_to(path).await;

        if result.is_ok() {
            _ = set_permissions(file.path().unwrap(), Permissions::from_mode(0o644));
        } else {
            errors.push(i);
        }
    }

    if errors.is_empty() {
        Redirect::to(uri!("/result"))
    } else {
        let error = format!(
            "Error uploading {} files with the following indexes {errors:?}",
            errors.len()
        );
        Redirect::to(uri!(error(error)))
    }
}

fn filename(file: &mut TempFile, i: usize) -> String {
    let Some(filename) = file.raw_name() else {
        return i.to_string();
    };
    sanitize(filename.dangerous_unsafe_unsanitized_raw())
}

#[get("/result?<error>")]
#[allow(unused)]
fn error(error: &str) {}

#[derive(FromForm)]
struct Upload<'r> {
    files: Vec<TempFile<'r>>,
    #[field(name = "data-usage-agreement-radio")]
    privacy: Privacy,
}

#[derive(Clone, Copy, PartialEq, FromFormField)]
enum Privacy {
    #[field(value = "can-be-public")]
    Public,
    #[field(value = "keep-private")]
    Private,
}
