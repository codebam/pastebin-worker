use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let method = req.method().to_string();
    let method_str = method.as_str();
    let mut req_mut = req.clone_mut().unwrap();
    let data = req_mut.text().await.unwrap();
    match method_str {
        "GET" => {
            let _result = env.kv("rust_worker")
                .map_err(|e| console_log!("{}", e)).unwrap()
                .get(req.path().as_str())
                .text().await
                .map_err(|e| console_log!("{}", e)).unwrap()
                .unwrap_or_else(|| "404".to_string());
            return match _result.as_str() {
                "404" => Response::error(_result, 404),
                &_ => {
                    if req.path().as_str() == "/" {
                        return Response::from_html(_result)
                    }
                    if req.path().as_str().trim_end_matches(".*").len() > 0 {
                        return Response::from_body(
                            ResponseBody::Body(_result.as_str().as_bytes().to_vec())
                        )
                    }
                    Response::ok(_result)
                } 
            }
        }
        "POST" | "PUT" => {
            if req.path().as_str() == "/" {
                return Response::ok("cannot update /")
            }
            let _result = env.kv("rust_worker")
                .map_err(|e| console_log!("{}", e)).unwrap()
                .put(req.path().as_str(), data.clone())
                .map_err(|e| console_log!("{}", e)).unwrap()
                .execute().await;
            Response::ok(data)
        },
        "DELETE" => {
            let _result = env.kv("rust_worker")
                .map_err(|e| console_log!("{}", e)).unwrap()
                .delete(req.path().as_str()).await;
            Response::ok("deleted")
        }
        &_ => Response::ok(method)
    }
}
