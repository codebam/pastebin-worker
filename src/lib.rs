use worker::*;

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let method = req.method().to_string();
    let method_str = method.as_str();
    let mut req_mut = req.clone_mut().unwrap();
    let data = req_mut.text().await.unwrap();
    match method_str {
        "POST" | "PUT" => {
            let _result = env.kv("rust_worker")
                .map_err(|e| console_log!("{}", e)).unwrap()
                .put(req.path().as_str(), data.clone())
                .map_err(|e| console_log!("{}", e)).unwrap()
                .execute().await;
            Response::ok(data)
        },
        "GET" => {
            let _result = env.kv("rust_worker")
                .map_err(|e| console_log!("{}", e)).unwrap()
                .get(req.path().as_str())
                .text().await
                .map_err(|e| console_log!("{}", e)).unwrap()
                .unwrap_or_else(|| "404".to_string());
            Response::ok(_result)
        }
        &_ => Response::ok(method)
    }
}
