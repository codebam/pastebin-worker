#![feature(try_blocks)]
#![feature(path_file_prefix)]
use base64::{engine::general_purpose, Engine as _};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305,
};
use lz4_flex::block::{compress_prepend_size, decompress_size_prepended};
use rand::{distributions::Alphanumeric, Rng};
use regex::Regex;
use serde_json;
use std::{ffi::OsStr, path::Path};
use urlencoding::decode;
use worker::*;

extern crate console_error_panic_hook;

async fn post_put(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let mut req_mut = _req.clone_mut().map_err(|e| console_log!("{}", e)).unwrap();
    let form_data = req_mut.form_data().await.unwrap();
    let form_entry = form_data.get("upload").unwrap_or_else(|| {
        form_data
            .get("paste")
            .unwrap_or_else(|| FormEntry::File(File::new("", "")))
    });
    let random_name: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    let file = match form_entry {
        FormEntry::Field(form_entry) => File::new(form_entry.into_bytes(), "paste"),
        FormEntry::File(form_entry) => form_entry,
    };
    let filename = random_name;
    let path = Path::new(filename.as_str())
        .file_prefix()
        .unwrap_or_else(|| OsStr::new(""))
        .to_str()
        .unwrap_or_else(|| "");
    let path_str = path;
    if path_str == "/" {
        return Response::ok("cannot update /");
    }
    let compressed = compress_prepend_size(&file.bytes().await.unwrap());
    let b64 = general_purpose::STANDARD.encode(compressed);
    let _result = ctx
        .kv("pastebin")
        .unwrap()
        .put(path_str, b64)
        .map_err(|e| console_log!("{}", e))
        .unwrap()
        .execute()
        .await;
    let url = _req.url().unwrap();
    let redirect = String::from(url) + path_str + ".txt";
    let redirect_url = Url::parse(redirect.as_str()).unwrap();
    Response::redirect(redirect_url)
}

async fn post_encrypted(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let key = ChaCha20Poly1305::generate_key(&mut OsRng);
    let keytext = general_purpose::STANDARD.encode(serde_json::to_string(&key).unwrap());
    let cipher = ChaCha20Poly1305::new(&key);
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng); // 96-bits; unique per message
    let noncetext = general_purpose::STANDARD.encode(serde_json::to_string(&nonce).unwrap());
    let mut req_mut = _req.clone_mut().map_err(|e| console_log!("{}", e)).unwrap();
    let form_data = req_mut.form_data().await.unwrap();
    let form_entry = form_data.get("upload").unwrap_or_else(|| {
        form_data
            .get("paste")
            .unwrap_or_else(|| FormEntry::File(File::new("", "")))
    });
    let file = match form_entry {
        FormEntry::Field(form_entry) => File::new(form_entry.into_bytes(), "paste"),
        FormEntry::File(form_entry) => form_entry,
    };
    let compressed = compress_prepend_size(&file.bytes().await.unwrap());
    let ciphertext = cipher
        .encrypt(
            &nonce,
            general_purpose::STANDARD
                .encode(compressed)
                .as_bytes()
                .as_ref(),
        )
        .unwrap();

    let random_name: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    let filename = random_name;
    let path = Path::new(filename.as_str())
        .file_prefix()
        .unwrap_or_else(|| OsStr::new(""))
        .to_str()
        .unwrap_or_else(|| "");
    let path_str = path;
    if path_str == "/" {
        return Response::ok("cannot update /");
    }
    let b64 = general_purpose::STANDARD.encode(&ciphertext);
    let _result = ctx
        .kv("pastebin")
        .unwrap()
        .put(path_str, b64)
        .map_err(|e| console_log!("{}", e))
        .unwrap()
        .execute()
        .await;
    let url = _req.url().unwrap();
    let redirect = String::from(url)
        + "/decrypt/"
        + urlencoding::encode(&keytext).to_string().as_str()
        + "/"
        + urlencoding::encode(&noncetext).to_string().as_str()
        + "/"
        + path_str
        + ".txt";
    let redirect_url = Url::parse(redirect.as_str()).unwrap();
    Response::redirect(redirect_url)
}

async fn delete(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let empty_string = String::new();
    let file = ctx.param("file").unwrap_or_else(|| &empty_string).as_str();
    let _result = ctx
        .kv("pastebin")
        .map_err(|e| console_log!("{}", e))
        .unwrap()
        .delete(file)
        .await;
    let mut headers = Headers::new();
    let _result = headers.append("Access-Control-Allow-Origin", "*").unwrap();
    Ok(Response::with_headers(
        Response::ok("deleted").unwrap(),
        headers,
    ))
}

async fn get_index(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let result = ctx
        .kv("pastebin")
        .unwrap()
        .get("/")
        .text()
        .await
        .map_err(|e| console_log!("{}", e))
        .unwrap()
        .unwrap_or_else(|| String::from("404"));
    Response::from_html(result)
}

async fn get(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let reqpath = String::from(decode(ctx.param("file").unwrap()).expect("UTF-8"));
    let path = Path::new(reqpath.as_str());
    let name = path
        .file_prefix()
        .unwrap_or_else(|| OsStr::new(""))
        .to_str()
        .unwrap_or_else(|| "");
    let result = ctx
        .kv("pastebin")
        .unwrap()
        .get(name)
        .text()
        .await
        .map_err(|e| console_log!("{}", e))
        .unwrap_or_else(|_| Some(String::from("404")))
        .unwrap_or_else(|| String::from("404"));
    let body = general_purpose::STANDARD
        .decode(result.as_str())
        .unwrap_or_else(|_| "".as_bytes().to_vec());
    let decompressed =
        decompress_size_prepended(body.as_slice()).unwrap_or_else(|_| "".as_bytes().to_vec());
    return match result.as_str() {
        "404" => Response::error(result, 404),
        &_ => {
            let mime = mime_guess::from_path(path)
                .first()
                .unwrap_or_else(|| mime_guess::from_ext("txt").first().unwrap());
            let response = Response::from_body(ResponseBody::Body(decompressed));
            let mut headers = Headers::new();
            let _result = headers
                .append("Content-type", mime.to_string().as_str())
                .unwrap();
            Ok(Response::with_headers(response.unwrap(), headers))
        }
    };
}

async fn get_encrypted(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let key_param = ctx.param("key").unwrap().to_owned();
    let key_decoded: String = urlencoding::decode(key_param.as_str()).unwrap().to_string();
    let key_decoded2 =
        String::from_utf8(general_purpose::STANDARD.decode(key_decoded).unwrap()).unwrap();
    let key = serde_json::from_str(key_decoded2.as_str()).unwrap();
    let nonce_param = ctx.param("nonce").unwrap().to_owned();
    let nonce_decoded: String = urlencoding::decode(nonce_param.as_str())
        .unwrap()
        .to_string();
    let nonce_decoded2 =
        String::from_utf8(general_purpose::STANDARD.decode(nonce_decoded).unwrap()).unwrap();
    let cipher = ChaCha20Poly1305::new(&key);
    let nonce = serde_json::from_str(nonce_decoded2.as_str()).unwrap();
    let reqpath = String::from(decode(ctx.param("file").unwrap()).expect("UTF-8"));
    let path = Path::new(reqpath.as_str());
    let name = path
        .file_prefix()
        .unwrap_or_else(|| OsStr::new(""))
        .to_str()
        .unwrap_or_else(|| "");
    let result = ctx
        .kv("pastebin")
        .unwrap()
        .get(name)
        .text()
        .await
        .map_err(|e| console_log!("{}", e))
        .unwrap_or_else(|_| Some(String::from("404")))
        .unwrap_or_else(|| String::from("404"));
    let body = general_purpose::STANDARD
        .decode(result.as_str())
        .unwrap_or_else(|_| "".as_bytes().to_vec());
    let plaintext = cipher.decrypt(&nonce, body.as_ref()).unwrap_or_else(|_| {
        console_log!("failed to decrypt");
        vec![]
    });
    let plaintext_decoded = general_purpose::STANDARD.decode(plaintext).unwrap();
    let decompressed =
        decompress_size_prepended(&plaintext_decoded).unwrap_or_else(|_| "".as_bytes().to_vec());
    return match result.as_str() {
        "404" => Response::error(result, 404),
        &_ => {
            let mime = mime_guess::from_path(path)
                .first()
                .unwrap_or_else(|| mime_guess::from_ext("txt").first().unwrap());
            let response = Response::from_body(ResponseBody::Body(decompressed));
            let mut headers = Headers::new();
            let _result = headers
                .append("Content-type", mime.to_string().as_str())
                .unwrap();
            Ok(Response::with_headers(response.unwrap(), headers))
        }
    };
}

async fn get_list(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let result = ctx
        .kv("pastebin")
        .unwrap()
        .list()
        .execute()
        .await
        .unwrap()
        .keys
        .iter()
        .cloned()
        .map(|key| key.name + "\n")
        .collect::<String>();
    Response::ok(result)
}

async fn get_raw(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let file_param = ctx.param("file").unwrap().to_owned();
    let mime = mime_guess::from_path(file_param.clone())
        .first()
        .unwrap_or_else(|| mime_guess::from_ext("txt").first().unwrap());
    let mut headers = Headers::new();
    let result = ctx
        .kv("pastebin")
        .unwrap()
        .get(file_param.as_str())
        .text()
        .await
        .map_err(|e| console_log!("{}", e))
        .unwrap()
        .unwrap_or_else(|| String::from("404"));
    let response = Response::from_body(ResponseBody::Body(result.into_bytes()));
    let _result = headers
        .append("Content-type", mime.to_string().as_str())
        .unwrap();
    Ok(Response::with_headers(response.unwrap(), headers))
}

async fn get_highlight(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let result = ctx
        .kv("pastebin")
        .unwrap()
        .get("highlight.html")
        .text()
        .await
        .map_err(|e| console_log!("{}", e))
        .unwrap()
        .unwrap_or_else(|| String::from("404"));
    Response::from_html(result)
}

async fn search(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let search_param = ctx.param("pattern").unwrap().to_owned();
    let list = ctx
        .kv("pastebin")
        .unwrap()
        .list()
        .execute()
        .await
        .unwrap()
        .keys
        .iter()
        .cloned()
        .map(|key| key.name + "\n")
        .collect::<String>()
        .split("\n")
        .collect::<Vec<&str>>()
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>();
    let re = Regex::new(search_param.as_str()).unwrap();
    let mut results: Vec<String> = vec![];
    for file in list {
        let contents = ctx
            .kv("pastebin")
            .unwrap()
            .get(file.as_str())
            .text()
            .await
            .unwrap_or_else(|_| Some("".to_string()))
            .unwrap_or_else(|| "".to_string());
        let body = general_purpose::STANDARD
            .decode(contents.as_str())
            .unwrap_or_else(|_| "".as_bytes().to_vec());
        let decompressed =
            decompress_size_prepended(body.as_slice()).unwrap_or_else(|_| "".as_bytes().to_vec());
        match re.captures(
            String::from_utf8(decompressed)
                .unwrap_or_else(|_| "".to_string())
                .as_str(),
        ) {
            Some(_) => results.append(&mut vec![file]),
            None => {}
        }
    }
    let results_string = results
        .iter()
        .cloned()
        .map(|key| key + "\n")
        .collect::<String>();
    Response::ok(results_string)
}

async fn get_search(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let result = ctx
        .kv("pastebin")
        .unwrap()
        .get("search.html")
        .text()
        .await
        .map_err(|e| console_log!("{}", e))
        .unwrap()
        .unwrap_or_else(|| String::from("404"));
    Response::from_html(result)
}

async fn get_term(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let result = ctx
        .kv("pastebin")
        .unwrap()
        .get("term.html")
        .text()
        .await
        .map_err(|e| console_log!("{}", e))
        .unwrap()
        .unwrap_or_else(|| String::from("404"));
    Response::from_html(result)
}

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    let router = Router::new();
    router
        .get_async("/", get_index)
        .get_async("/list", get_list)
        .get_async("/encrypt/decrypt/:key/:nonce/:file", get_encrypted)
        .get_async("/:file", get)
        .get_async("/term", get_term)
        .get_async("/files", get_search)
        .get_async("/raw/:file", get_raw)
        .get_async("/search/:pattern", search)
        .get_async("/highlight/:file", get_highlight)
        .post_async("/", post_put)
        .put_async("/", post_put)
        .post_async("/encrypt", post_encrypted)
        .delete_async("/:file", delete)
        .get_async("/delete/:file", delete)
        .or_else_any_method_async("/", |req, _ctx| async move {
            Response::redirect(req.url().unwrap())
        })
        .run(req, env)
        .await
}
