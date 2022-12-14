use rusthttpserver;
use rusthttpserver::request::Request;
use rusthttpserver::response::Response;

fn main() {
    // Create a rusthttpserver app with 2 worker threads
    let mut app = rusthttpserver::RustHTTPServer::new(2);
    // Use a directory called public in the project root to serve static files
    app.public("public");

    app.middle(
        "/post/",
        |req: Request, mut res: Response| -> (Request, Response, bool) {
            if req.method == "POST" {
                if req.body.len() > 0 {
                    return (req, res, true);
                }
                res.status(400);
            }
            return (req, res, false);
        },
    );

    // Redirect
    app.route("/", |req: Request, mut res: Response| -> Response {
        if req.method == "GET" {
            res.status(301);
            res.header("Location", "/index.html");
        }
        return res;
    });

    // GET with params
    app.route("/user/", |req: Request, mut res: Response| -> Response {
        let param_keys = vec!["name", "age"];
        if req.method == "GET" {
            match req.contains_params(param_keys) {
                Some(missing_key) => {
                    res.status(400);
                    res.body(format!("Missing parameter: {}", missing_key));
                    return res;
                }
                None => {}
            }
            res.status(200);
            res.body(format!(
                "<h1>Hello {}, age {}!</h1>",
                req.params.get("name").unwrap(),
                req.params.get("age").unwrap(),
            ));
            return res;
        } else {
            // Default response is 404
            return res;
        };
    });

    // Add a POST endpoint to /post
    app.route("/post/", |req: Request, mut res: Response| -> Response {
        // RustHTTPServer does not have JSON serilization built inn,
        // if you want to parse JSON consider combining rusthttpserver with serde_json (https://crates.io/crates/serde_json)
        if req.method == "POST" {
            if req.headers["content-type"] == "application/json" {
                println!("{}", String::from_utf8_lossy(&req.body));
            }
            res.status(200);
            res.body("{\"message\": \"Hello World!\"}");
            // HTTP headers can be added like this
            res.header("content-type", "application/json");
            return res;
        } else {
            return res;
        };
    });

    // Bind the rusthttpserver app to port 3000 on the local IP adress
    let err = app.bind("127.0.0.1:3000");
    println!("{}", err);
}
