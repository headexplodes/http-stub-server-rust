use std::ascii::AsciiExt;

pub mod service;

struct StubParam {
    name: String,
    value: String
}

trait StubMessage {
    fn headers(&self) -> &Vec<StubParam>;
    fn body(&self) -> Option<&Vec<u8>>;

    /*
      def getHeader(name: String): Option[String] =
        headers.find(_.name.equalsIgnoreCase(name)).map(_.value)
      def getHeaders(name: String): Seq[String] =
        headers.filter(_.name.equalsIgnoreCase(name)).map(_.value)

      def addHeader(name: String, value: String): T =
        copyWith(headers :+ StubParam(name, value))
      def removeHeader(name: String): T =
        copyWith(headers.filterNot(_.name.equalsIgnoreCase(name)))
      def setHeader(name: String, value: String): T#T =
        removeHeader(name).addHeader(name, value)
    */
}

#[derive(Default)]
struct StubRequest {
    method: Option<String>,
    path: Option<String>,
    params: Vec<StubParam>,
    headers: Vec<StubParam>,
    body: Option<Vec<u8>>,
    body_type: Option<String>
}

impl StubRequest {
    fn get_param(&self, name: &str) -> Option<&str> {
        self.params.iter().find(|_p| _p.name.eq_ignore_ascii_case(name)).as_ref().map(|_p| &_p.value as &str)
    }
    fn get_params(&self, name: &str) -> Vec<&str> {
        self.params.iter().filter(|_p| _p.name.eq_ignore_ascii_case(name)).map(|_p| &_p.value as &str).collect()
    }
}

impl StubMessage for StubRequest {
    fn headers(&self) -> &Vec<StubParam> {
        &self.headers
    }
    fn body(&self) -> Option<&Vec<u8>> {
        self.body.as_ref()
    }
}

#[derive(Default)]
struct StubResponse {
    status: u16,
    headers: Vec<StubParam>,
    body: Option<Vec<u8>>
}

impl StubMessage for StubResponse {
    fn headers(&self) -> &Vec<StubParam> {
        &self.headers
    }
    fn body(&self) -> Option<&Vec<u8>> {
        self.body.as_ref()
    }
}

struct StubExchange {
    request: StubRequest,
    response: StubResponse,
    delay: Option<u32>
}
