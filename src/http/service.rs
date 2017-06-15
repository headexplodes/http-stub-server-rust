use futures;
use hyper;
use hyper::StatusCode;
use hyper::Method;
use hyper::server::{Request, Response, Service};
use hyper::header::{ContentLength, ContentType};
use futures::Future;
use futures::sync::mpsc;
use futures::sink::Sink;
use serde_json;
use serde::Serialize;

type FutureResult = futures::future::FutureResult<Response, hyper::Error>;

static MSG_NOT_FOUND: &'static str = "Not found";
static MSG_METHOD_NOT_ALLOWED: &'static str = "Method not allowed";
static MSG_INTERNAL_SERVER_ERROR: &'static str = "Internal server error";

const CARGO_PKG_VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub struct HttpService {
    // pub shutdown_msg: String,
    pub shutdown_promise: mpsc::Sender<()>
}

pub fn split_path(path: &str) -> Vec<&str> {
    path.trim_matches('/')
        .split("/")
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use http::service::split_path;

    #[test]
    fn test_split_path() {
        assert_eq!(split_path(""), vec![] as Vec<&str>);
        assert_eq!(split_path("/"), vec![] as Vec<&str>);
        assert_eq!(split_path("/foo"), vec!["foo"]);
        assert_eq!(split_path("/foo/"), vec!["foo"]);
        assert_eq!(split_path("/foo/bar"), vec!["foo", "bar"]);
    }
}

fn ok(message: &str) -> FutureResult {
    futures::future::ok(
        Response::new()
            .with_status(StatusCode::Ok)
            .with_header(ContentType::plaintext())
            .with_header(ContentLength(message.len() as u64))
            .with_body(message.to_owned())
    )
}

fn ok_json<T: Serialize>(obj: &T) -> FutureResult {
    return match serde_json::to_string(&obj) {
        Err(e) => {
            error!("Error serialising JSON response: {}", e);
            internal_server_error()
        }
        Ok(body) => futures::future::ok(
            Response::new()
                .with_status(StatusCode::Ok)
                .with_header(ContentType::json())
                .with_header(ContentLength(body.len() as u64))
                .with_body(body)
        ),
    };
}

fn internal_server_error() -> FutureResult {
    futures::future::ok(
        Response::new()
            .with_status(StatusCode::InternalServerError)
            .with_header(ContentType::plaintext())
            .with_header(ContentLength(MSG_INTERNAL_SERVER_ERROR.len() as u64))
            .with_body(MSG_INTERNAL_SERVER_ERROR)
    )
}

fn not_found() -> FutureResult {
    futures::future::ok(
        Response::new()
            .with_status(StatusCode::NotFound)
            .with_header(ContentType::plaintext())
            .with_header(ContentLength(MSG_NOT_FOUND.len() as u64))
            .with_body(MSG_NOT_FOUND)
    )
}

fn method_not_allowed() -> FutureResult {
    futures::future::ok(
        Response::new()
            .with_status(StatusCode::MethodNotAllowed)
            .with_header(ContentType::plaintext())
            .with_header(ContentLength(MSG_METHOD_NOT_ALLOWED.len() as u64))
            .with_body(MSG_METHOD_NOT_ALLOWED)
    )
}

#[derive(Serialize)]
struct VersionResponse {
    version: String
}

// TODO: https://hyper.rs/guides/server/echo/

impl HttpService {
    fn handle(&self, req: &Request, path: &[&str]) -> FutureResult {
        match path {
            &["_control", ref tail..] => self.handle_control(req, tail),
            _ => ok("TODO: match request ..."),
        }
    }

    fn handle_control(&self, req: &Request, path: &[&str]) -> FutureResult {
        match path {
            &["shutdown"] => self.handle_control_shutdown(req),
            &["version"] => self.handle_control_version(req),
            &["responses", ref tail..] => self.handle_control_responses(req, tail),
            &["requests", ref tail..] => self.handle_control_requests(req, tail),
            _ => not_found(),
        }
    }

    fn handle_control_shutdown(&self, req: &Request) -> FutureResult {
        match *req.method() {
            // Method::Post => ok("TODO: POST /_control/shutdown"),
            Method::Post => {
                if self.shutdown_promise.clone().send(()).wait().is_err() {
                    warn!("shutdown(): Error queueing shutdown message"); // other end may have stopped listening
                }

                debug!("Shutdown endpoint called, triggering shutdown...");

                let shutdown_msg = "{\"message\": \"Shutdown triggered\"}";

                futures::future::ok(
                    Response::new()
                        .with_status(StatusCode::Accepted)
                        .with_header(ContentType::plaintext())
                        .with_header(ContentLength(shutdown_msg.len() as u64))
                        .with_body(shutdown_msg))
                        // .with_body(self.shutdown_msg.clone()))
            }
            _ => method_not_allowed(),
        }
    }

    fn handle_control_version(&self, req: &Request) -> FutureResult {
        let version = VersionResponse { version: CARGO_PKG_VERSION.to_owned() };
        match *req.method() {
            Method::Get => ok_json(&version),
            _ => method_not_allowed(),
        }
    }

    fn handle_control_responses(&self, req: &Request, path: &[&str]) -> FutureResult {
        match path {
            &[] => {
                match *req.method() {
                    Method::Get => ok("TODO: GET /_control/responses"),
                    Method::Delete => ok("TODO: DELETE /_control/responses"),
                    Method::Post => ok("TODO: POST /_control/responses"),
                    _ => method_not_allowed(),
                }
            }
            &[id] => {
                match *req.method() {
                    Method::Get => ok("TODO: GET /_control/responses/{id}"),
                    Method::Delete => ok("TODO: DELETE /_control/responses/{id}"),
                    _ => method_not_allowed(),
                }
            }
            _ => not_found(),
        }
    }

    fn handle_control_requests(&self, req: &Request, path: &[&str]) -> FutureResult {
        match path {
            &[] => {
                match *req.method() {
                    Method::Get => ok("TODO: GET /_control/requests"), // TODO: 'wait' parameter
                    Method::Delete => ok("TODO: DELETE /_control/requests"),
                    _ => method_not_allowed(),
                }
            }
            &[id] => {
                match *req.method() {
                    Method::Get => ok("TODO: GET /_control/requests/{id}"),
                    Method::Delete => ok("TODO: DELETE /_control/requests/{id}"),
                    _ => method_not_allowed(),
                }
            }
            _ => not_found(),
        }
    }
}

impl Service for HttpService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    //    type Future = futures::future::FutureResult<Self::Response, Self::Error>;
    type Future = FutureResult;

    fn call(&self, req: Request) -> Self::Future {
        self.handle(&req, &split_path(&req.path())[..])
    }
}

/*

class AppPlan(server: Server) extends cycle.Plan with cycle.ThreadPool with ServerErrorResponse {
  def intent = {
    case req @ Path(Seg("_control" :: "shutdown" :: Nil)) => req match {
      case _ =>
        Main.http.get.stop()
        ResponseString("Shutting Down")
    }
    case req @ Path(Seg("_control" :: "version" :: Nil)) => req match {
      case _ =>
        JsonResponse(JsonUtils.serialize(Map("version" -> BuildInfo.version))) // 'BuildInfo' object is auto-generated
    }
    case req @ Path(Seg("_control" :: "responses" :: Nil)) => req match {
      case GET(_) => server.getResponses
      case DELETE(_) => server.deleteResponses
      case POST(_) => server.addResponse(req)
    }
    case req @ Path(Seg("_control" :: "responses" :: id :: Nil)) => req match {
      case GET(_) => server.getResponse(id.toInt)
      case DELETE(_) => server.deleteResponse(id.toInt)
    }
    case req @ Path(Seg("_control" :: "requests" :: Nil)) => req match {
      case GET(_) => req match {
        case Params(WaitParam(wait)) => server.findRequests(req, wait)
        case _ => server.findRequests(req)
      }
      case DELETE(_) => server.deleteRequests
    }
    case req @ Path(Seg("_control" :: "requests" :: id :: Nil)) => req match {
      case GET(_) => server.getRequest(id.toInt)
      case DELETE(_) => server.deleteRequest(id.toInt)
    }
    case req @ _ => {
      server.matchRequest(req)
    }
  }
}

case class JsonResponse(json: String)
  extends ComposeResponse(JsonContent ~> ResponseString(json))

case class EmptyOk(any: Any)
  extends ComposeResponse(Ok)

case class StubUnfilteredResponse(result: StubResponse) extends Responder[Any] {
  def respond(res: HttpResponse[Any]) {
    val out = res.outputStream
    try {
      res.status(result.status)
      result.headers.foreach {
        h => res.header(h.name, h.value)
      }
      result.body match {
        case Some(body: String) => IOUtils.write(body, out)
        case Some(body: AnyRef) => JsonUtils.serialize(out, body) // assume deserialised JSON (ie, a Map or List)
        case None =>
      }
    } finally {
      out.close()
    }
  }
}

class Server(paths: Seq[File]) {
  import Transformer._

  val service = new StubService
  val jsonService = new JsonServiceInterface(service)
  val fileSource = new FileSource(paths, service, jsonService).loadInitialFiles().watchFolders()

  private def handleNotFound[T >: ResponseFunction[Any]](body: => T): T =
    try {
      body
    } catch {
      case NotFoundException(message) =>
        NotFound ~> ResponseString(message)
    }

  def findRequests(req: HttpRequest[_]) =
    JsonResponse(jsonService.findRequests(createFilter(req)))
  def findRequests(req: HttpRequest[_], wait: Int) =
    JsonResponse(jsonService.findRequests(createFilter(req), wait))

  def getRequest(index: Int) = handleNotFound {
    JsonResponse(jsonService.getRequest(index))
  }

  def deleteRequests() =
    EmptyOk(jsonService.deleteRequests)
  def deleteRequest(index: Int) = handleNotFound {
    EmptyOk(jsonService.deleteRequest(index))
  }

  private def createFilter(req: HttpRequest[_]) =
    RequestFilterBuilder.makeFilter(Transformer.parseQuery(req))

  def getResponses() =
    JsonResponse(jsonService.getResponses)
  def getResponse(index: Int) = handleNotFound {
    JsonResponse(jsonService.getResponse(index))
  }

  def deleteResponses() =
    EmptyOk(jsonService.deleteResponses)
  def deleteResponse(index: Int) = handleNotFound {
    EmptyOk(jsonService.deleteResponse(index))
  }

  def addResponse(req: HttpRequest[_]) =
    EmptyOk(jsonService.addResponse(req.inputStream))

  def matchRequest(req: HttpRequest[ReceivedMessage]) = {
    val result = service.findMatch(toStubRequest(req))
    if (result.matchFound) {
      result.delay.foreach(t => Thread.sleep(t)) // sleep if delay given
      StubUnfilteredResponse(result.response.get)
    } else {
      NotFound ~> ResponseString("No stubbed response found")
    }
  }
}

object WaitParam extends Params.Extract(
  "wait", Params.first ~> Params.int ~> Params.pred { _ > 0 }
)

}
*/