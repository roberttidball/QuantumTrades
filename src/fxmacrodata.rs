use serde_json::Value;

pub const API_KEY_HEADER: &str = "X-API-Key";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FxMacroDataEndpoint {
    DataCatalogue,
    Announcements,
    LatestAnnouncements,
    AnnouncementChanges,
    Calendar,
    Predictions,
    Forex,
    Cot,
    Commodity,
    CommoditiesLatest,
    Curves,
    CurveProxies,
    ForwardCurves,
    RateDifferentials,
    ForwardDifferentials,
    MarketSessions,
    RiskSentiment,
    News,
    PressReleases,
    Graphql,
    Custom,
}

#[derive(Clone, Debug)]
pub struct FxMacroDataRequest {
    pub endpoint: FxMacroDataEndpoint,
    pub currency: Option<String>,
    pub indicator: Option<String>,
    pub base: Option<String>,
    pub quote: Option<String>,
    pub path: Option<String>,
    pub params: Vec<(String, String)>,
    pub body: Option<Value>,
}

impl FxMacroDataRequest {
    pub fn new(endpoint: FxMacroDataEndpoint) -> Self {
        Self {
            endpoint,
            currency: None,
            indicator: None,
            base: None,
            quote: None,
            path: None,
            params: Vec::new(),
            body: None,
        }
    }

    /// Select one page of a list endpoint. List endpoints return 20 rows by
    /// default and at most 100 per request, newest first; request the next
    /// page with the response's `pagination.next_offset` while
    /// `pagination.has_more` is true.
    pub fn page(mut self, limit: u32, offset: u32) -> Self {
        self.params
            .retain(|(key, _)| key != "limit" && key != "offset");
        self.params
            .push(("limit".to_owned(), limit.clamp(1, 100).to_string()));
        self.params.push(("offset".to_owned(), offset.to_string()));
        self
    }
}

#[derive(Clone)]
pub struct FxMacroDataClient {
    base_url: String,
    api_key: Option<String>,
}

impl Default for FxMacroDataClient {
    fn default() -> Self {
        Self::new(
            std::env::var("FXMACRODATA_API_KEY")
                .or_else(|_| std::env::var("FXMD_API_KEY"))
                .ok(),
        )
    }
}

impl FxMacroDataClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(api_key, "https://api.fxmacrodata.com/v1")
    }

    pub fn with_base_url(api_key: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            api_key: api_key.filter(|key| !key.trim().is_empty()),
        }
    }

    /// Header sent with each request, or `None` when no key is set.
    pub fn api_key_header(&self) -> Option<(&'static str, &str)> {
        self.api_key.as_deref().map(|key| (API_KEY_HEADER, key))
    }

    pub fn fetch_json(&self, request: FxMacroDataRequest) -> crate::Result<Value> {
        let url = self.build_url(&request)?;
        let is_post = matches!(request.endpoint, FxMacroDataEndpoint::Graphql)
            || (matches!(request.endpoint, FxMacroDataEndpoint::Custom) && request.body.is_some());
        let mut http = if is_post {
            ureq::post(&url)
        } else {
            ureq::get(&url)
        };
        if let Some((name, key)) = self.api_key_header() {
            http = http.set(name, key);
        }
        let response = if is_post {
            http.send_json(request.body.unwrap_or(Value::Null))?
        } else {
            http.call()?
        };
        Ok(response.into_json()?)
    }

    pub fn calendar(&self, currency: &str) -> FxMacroDataRequest {
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::Calendar);
        request.currency = Some(currency.to_owned());
        request
    }

    pub fn predictions(&self, currency: &str, indicator: &str) -> FxMacroDataRequest {
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::Predictions);
        request.currency = Some(currency.to_owned());
        request.indicator = Some(indicator.to_owned());
        request
    }

    pub fn forex(&self, base: &str, quote: &str) -> FxMacroDataRequest {
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::Forex);
        request.base = Some(base.to_owned());
        request.quote = Some(quote.to_owned());
        request
    }

    pub fn build_url(&self, request: &FxMacroDataRequest) -> crate::Result<String> {
        let params = &request.params;
        let mut url = format!("{}{}", self.base_url, self.path(request)?);
        if !params.is_empty() {
            url.push('?');
            url.push_str(
                &params
                    .iter()
                    .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
                    .collect::<Vec<_>>()
                    .join("&"),
            );
        }
        Ok(url)
    }

    fn path(&self, request: &FxMacroDataRequest) -> crate::Result<String> {
        let path = match request.endpoint {
            FxMacroDataEndpoint::DataCatalogue => {
                format!(
                    "/data_catalogue/{}",
                    segment(&request.currency, "currency")?
                )
            }
            FxMacroDataEndpoint::Announcements => format!(
                "/announcements/{}/{}",
                segment(&request.currency, "currency")?,
                segment(&request.indicator, "indicator")?
            ),
            FxMacroDataEndpoint::LatestAnnouncements => {
                format!(
                    "/announcements/{}/latest",
                    segment(&request.currency, "currency")?
                )
            }
            FxMacroDataEndpoint::AnnouncementChanges => "/announcements/changes".to_owned(),
            FxMacroDataEndpoint::Calendar => {
                format!("/calendar/{}", segment(&request.currency, "currency")?)
            }
            FxMacroDataEndpoint::Predictions => format!(
                "/predictions/{}/{}",
                segment(&request.currency, "currency")?,
                segment(&request.indicator, "indicator")?
            ),
            FxMacroDataEndpoint::Forex => format!(
                "/forex/{}/{}",
                segment(&request.base, "base")?,
                segment(&request.quote, "quote")?
            ),
            FxMacroDataEndpoint::Cot => format!("/cot/{}", segment(&request.currency, "currency")?),
            FxMacroDataEndpoint::Commodity => {
                format!("/commodities/{}", segment(&request.indicator, "indicator")?)
            }
            FxMacroDataEndpoint::CommoditiesLatest => "/commodities/latest".to_owned(),
            FxMacroDataEndpoint::Curves => {
                format!("/curves/{}", segment(&request.currency, "currency")?)
            }
            FxMacroDataEndpoint::CurveProxies => {
                format!("/curve_proxies/{}", segment(&request.currency, "currency")?)
            }
            FxMacroDataEndpoint::ForwardCurves => {
                format!(
                    "/forward_curves/{}",
                    segment(&request.currency, "currency")?
                )
            }
            FxMacroDataEndpoint::RateDifferentials => format!(
                "/rate_differentials/{}/{}",
                segment(&request.base, "base")?,
                segment(&request.quote, "quote")?
            ),
            FxMacroDataEndpoint::ForwardDifferentials => format!(
                "/forward_differentials/{}/{}",
                segment(&request.base, "base")?,
                segment(&request.quote, "quote")?
            ),
            FxMacroDataEndpoint::MarketSessions => "/market_sessions".to_owned(),
            FxMacroDataEndpoint::RiskSentiment => "/risk_sentiment".to_owned(),
            FxMacroDataEndpoint::News => {
                format!("/news/{}", segment(&request.currency, "currency")?)
            }
            FxMacroDataEndpoint::PressReleases => {
                format!(
                    "/press-releases/{}",
                    segment(&request.currency, "currency")?
                )
            }
            FxMacroDataEndpoint::Graphql => "/graphql".to_owned(),
            FxMacroDataEndpoint::Custom => request
                .path
                .as_deref()
                .map(|path| {
                    if path.starts_with('/') {
                        path.to_owned()
                    } else {
                        format!("/{path}")
                    }
                })
                .ok_or_else(|| "FXMacroData path is required".to_string())?,
        };
        Ok(path)
    }
}

impl std::fmt::Debug for FxMacroDataClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FxMacroDataClient")
            .field("base_url", &self.base_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

fn segment(value: &Option<String>, name: &str) -> crate::Result<String> {
    value
        .as_deref()
        .map(str::to_lowercase)
        .map(|value| encode(&value))
        .ok_or_else(|| format!("FXMacroData {name} is required").into())
}

fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{FxMacroDataClient, FxMacroDataEndpoint, FxMacroDataRequest};

    #[test]
    fn builds_macro_urls_without_key() {
        let client = FxMacroDataClient::with_base_url(
            Some("test-key".to_owned()),
            "https://api.fxmacrodata.com/v1/",
        );
        let mut request = client.predictions("USD", "non_farm_payrolls");
        request.params.push(("limit".to_owned(), "1".to_owned()));

        assert_eq!(
            client.build_url(&request).unwrap(),
            "https://api.fxmacrodata.com/v1/predictions/usd/non_farm_payrolls?limit=1"
        );
        assert_eq!(client.api_key_header(), Some(("X-API-Key", "test-key")));
    }

    #[test]
    fn page_sets_limit_and_offset() {
        let client = FxMacroDataClient::new(None);
        let request = client.forex("EUR", "USD").page(500, 200);

        assert_eq!(
            client.build_url(&request).unwrap(),
            "https://api.fxmacrodata.com/v1/forex/eur/usd?limit=100&offset=200"
        );
    }

    #[test]
    fn omits_header_without_key() {
        assert_eq!(FxMacroDataClient::new(None).api_key_header(), None);
        assert_eq!(
            FxMacroDataClient::new(Some(String::new())).api_key_header(),
            None
        );
    }

    #[test]
    fn builds_cross_currency_market_urls() {
        let client = FxMacroDataClient::new(Some("test-key".to_owned()));
        let mut request = FxMacroDataRequest::new(FxMacroDataEndpoint::RateDifferentials);
        request.base = Some("EUR".to_owned());
        request.quote = Some("USD".to_owned());
        request.params.push(("tenor".to_owned(), "2y".to_owned()));

        assert_eq!(
            client.build_url(&request).unwrap(),
            "https://api.fxmacrodata.com/v1/rate_differentials/eur/usd?tenor=2y"
        );
    }
}
