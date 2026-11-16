//! Licence reads from Microsoft Graph: which `GET`, and what is kept.
//!
//! Two read-only pages, decoded into typed values beside the user record
//! (like [`crate::group::MembersPage`]), never into it: the user schema and its
//! derived `$select` are unchanged.
//!
//! * [`ingest_licenses_page`] — `GET /users?$select=id,licenseAssignmentStates,assignedLicenses`:
//!   each user's assignment paths and effective SKUs.
//! * [`ingest_skus_page`] — `GET /subscribedSkus`: the tenant's capacity per SKU.
//!
//! What a decoded value never claims:
//!
//! * **Not read is not none.** A property absent or `null` in the page is
//!   `None`; an empty array is `Some(vec![])`. Only the second says the user
//!   has no licence.
//! * **A path is not a seat.** A user who holds one SKU both directly and
//!   through a group has two [`LicensePath`]s and one SKU.
//! * **A state is kept as read.** Text other than `Active`, `Disabled` or
//!   `Error` is [`PathState::Other`], never coerced to active.
//! * **Counts are exact.** Capacity counts are `i64` as Graph sends them, and
//!   [`SkuCapacity::free`] may be negative (more consumed than enabled).
//!
//! No cost, saving, expiry or departure is derived here.

use crate::AzError;
use ogar_dir_core::Guid128;
use serde_json::{Map, Value};

/// The read-only URL for users' licence properties.
pub fn licenses_url(page_size: u16) -> String {
    format!(
        "https://graph.microsoft.com/v1.0/users?$select=id,licenseAssignmentStates,assignedLicenses&$top={}",
        page_size.clamp(1, 999)
    )
}

/// The read-only URL for the tenant's subscribed SKUs.
pub fn skus_url() -> String {
    "https://graph.microsoft.com/v1.0/subscribedSkus".to_string()
}

/// The state of one assignment path, as Graph reported it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathState {
    /// `Active`.
    Active,
    /// `Disabled`.
    Disabled,
    /// `Error`: the path exists and the assignment failed.
    Error,
    /// Any other text, kept verbatim.
    Other(String),
}

impl PathState {
    fn parse(s: &str) -> PathState {
        match s {
            "Active" => PathState::Active,
            "Disabled" => PathState::Disabled,
            "Error" => PathState::Error,
            other => PathState::Other(other.to_string()),
        }
    }
}

/// One way a SKU reaches a user (one entry of `licenseAssignmentStates`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LicensePath {
    /// The SKU.
    pub sku: Guid128,
    /// The assigning group; `None` for a direct assignment.
    pub assigned_by_group: Option<Guid128>,
    /// The reported state.
    pub state: PathState,
    /// The reported `error` text, if present and not `null`.
    pub error: Option<String>,
    /// Service plans disabled on this path.
    pub disabled_plans: Vec<Guid128>,
}

/// One effective SKU of a user (one entry of `assignedLicenses`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveSku {
    /// The SKU.
    pub sku: Guid128,
    /// Service plans disabled for the user.
    pub disabled_plans: Vec<Guid128>,
}

/// One user's licence properties.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserLicenses {
    /// The Graph object id.
    pub user: Guid128,
    /// `licenseAssignmentStates`; `None` when not read.
    pub paths: Option<Vec<LicensePath>>,
    /// `assignedLicenses`; `None` when not read.
    pub effective: Option<Vec<EffectiveSku>>,
}

impl UserLicenses {
    /// The distinct SKUs over the assignment paths, in first-seen order;
    /// `None` when the paths were not read.
    pub fn skus(&self) -> Option<Vec<Guid128>> {
        let paths = self.paths.as_ref()?;
        let mut out: Vec<Guid128> = Vec::new();
        for p in paths {
            if !out.contains(&p.sku) {
                out.push(p.sku);
            }
        }
        Some(out)
    }
}

/// One page of users' licence properties.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LicensesPage {
    /// The users, in page order.
    pub users: Vec<UserLicenses>,
    /// `@odata.nextLink`, if Graph returned one.
    pub next_link: Option<String>,
}

/// Ingest one `GET /users?$select=id,licenseAssignmentStates,assignedLicenses` page.
///
/// A user without a parseable `id`, or any GUID that does not parse, is an
/// error, never dropped.
pub fn ingest_licenses_page(body: &str) -> Result<LicensesPage, AzError> {
    let v: Value = serde_json::from_str(body).map_err(|_| AzError::NotAPage)?;
    let items = page_items(&v)?;
    let mut page = LicensesPage {
        next_link: next_link(&v),
        ..LicensesPage::default()
    };
    for (i, item) in items.iter().enumerate() {
        let obj = item.as_object().ok_or(AzError::BadId(i))?;
        let user = guid_field(obj, "id").ok_or(AzError::BadId(i))?;
        let paths = match read_array(obj, i, "licenseAssignmentStates")? {
            None => None,
            Some(items) => Some(
                items
                    .iter()
                    .map(|s| decode_path(s, i))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        };
        let effective = match read_array(obj, i, "assignedLicenses")? {
            None => None,
            Some(items) => Some(
                items
                    .iter()
                    .map(|s| decode_effective(s, i))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        };
        page.users.push(UserLicenses {
            user,
            paths,
            effective,
        });
    }
    Ok(page)
}

/// One subscribed SKU's capacity, as Graph reported it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkuCapacity {
    /// The SKU.
    pub sku: Guid128,
    /// `skuPartNumber`, if present.
    pub part_number: Option<String>,
    /// `prepaidUnits.enabled`.
    pub enabled: i64,
    /// `prepaidUnits.suspended`.
    pub suspended: i64,
    /// `prepaidUnits.warning`.
    pub warning: i64,
    /// `consumedUnits`.
    pub consumed: i64,
    /// The SKU's service plans, by id.
    pub plans: Vec<Guid128>,
}

impl SkuCapacity {
    /// Enabled minus consumed. Negative when more is consumed than enabled.
    pub fn free(&self) -> i64 {
        self.enabled - self.consumed
    }
}

/// One `GET /subscribedSkus` page.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SkuPage {
    /// The SKUs, in page order.
    pub skus: Vec<SkuCapacity>,
    /// `@odata.nextLink`, if Graph returned one.
    pub next_link: Option<String>,
}

/// Ingest one `GET /subscribedSkus` page. A missing or non-integer count is
/// an error, never read as 0.
pub fn ingest_skus_page(body: &str) -> Result<SkuPage, AzError> {
    let v: Value = serde_json::from_str(body).map_err(|_| AzError::NotAPage)?;
    let items = page_items(&v)?;
    let mut page = SkuPage {
        next_link: next_link(&v),
        ..SkuPage::default()
    };
    for (i, item) in items.iter().enumerate() {
        let obj = item.as_object().ok_or(AzError::BadId(i))?;
        let sku = guid_field(obj, "skuId").ok_or(AzError::BadId(i))?;
        let part_number = match obj.get("skuPartNumber") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => return Err(AzError::BadType(i, "skuPartNumber")),
        };
        let prepaid = obj
            .get("prepaidUnits")
            .and_then(Value::as_object)
            .ok_or(AzError::BadType(i, "prepaidUnits"))?;
        let count = |o: &Map<String, Value>, k: &'static str| {
            o.get(k)
                .and_then(Value::as_i64)
                .ok_or(AzError::BadType(i, k))
        };
        let mut plans = Vec::new();
        if let Some(list) = read_array(obj, i, "servicePlans")? {
            for p in list {
                let id = p
                    .as_object()
                    .and_then(|o| guid_field(o, "servicePlanId"))
                    .ok_or(AzError::BadType(i, "servicePlanId"))?;
                plans.push(id);
            }
        }
        page.skus.push(SkuCapacity {
            sku,
            part_number,
            enabled: count(prepaid, "enabled")?,
            suspended: count(prepaid, "suspended")?,
            warning: count(prepaid, "warning")?,
            consumed: count(obj, "consumedUnits")?,
            plans,
        });
    }
    Ok(page)
}

fn decode_path(v: &Value, i: usize) -> Result<LicensePath, AzError> {
    let o = v
        .as_object()
        .ok_or(AzError::BadType(i, "licenseAssignmentStates"))?;
    let sku = guid_field(o, "skuId").ok_or(AzError::BadType(i, "skuId"))?;
    let assigned_by_group = match o.get("assignedByGroup") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => {
            Some(Guid128::parse(s).map_err(|_| AzError::BadType(i, "assignedByGroup"))?)
        }
        Some(_) => return Err(AzError::BadType(i, "assignedByGroup")),
    };
    let state = match o.get("state") {
        Some(Value::String(s)) => PathState::parse(s),
        _ => return Err(AzError::BadType(i, "state")),
    };
    let error = match o.get("error") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => return Err(AzError::BadType(i, "error")),
    };
    Ok(LicensePath {
        sku,
        assigned_by_group,
        state,
        error,
        disabled_plans: guid_list(o, i, "disabledPlans")?,
    })
}

fn decode_effective(v: &Value, i: usize) -> Result<EffectiveSku, AzError> {
    let o = v
        .as_object()
        .ok_or(AzError::BadType(i, "assignedLicenses"))?;
    Ok(EffectiveSku {
        sku: guid_field(o, "skuId").ok_or(AzError::BadType(i, "skuId"))?,
        disabled_plans: guid_list(o, i, "disabledPlans")?,
    })
}

fn page_items(v: &Value) -> Result<&Vec<Value>, AzError> {
    v.get("value")
        .and_then(Value::as_array)
        .ok_or(AzError::NotAPage)
}

/// `None` when the key is absent or `null`; an error when it is not an array.
fn read_array<'v>(
    o: &'v Map<String, Value>,
    i: usize,
    key: &'static str,
) -> Result<Option<&'v Vec<Value>>, AzError> {
    match o.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) => Ok(Some(a)),
        Some(_) => Err(AzError::BadType(i, key)),
    }
}

/// A list of GUID strings; absent or `null` is an empty list.
fn guid_list(o: &Map<String, Value>, i: usize, key: &'static str) -> Result<Vec<Guid128>, AzError> {
    let Some(items) = read_array(o, i, key)? else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .map(|g| {
            g.as_str()
                .and_then(|s| Guid128::parse(s).ok())
                .ok_or(AzError::BadType(i, key))
        })
        .collect()
}

fn guid_field(o: &Map<String, Value>, key: &str) -> Option<Guid128> {
    o.get(key)
        .and_then(Value::as_str)
        .and_then(|s| Guid128::parse(s).ok())
}

fn next_link(v: &Value) -> Option<String> {
    v.get("@odata.nextLink")
        .and_then(Value::as_str)
        .map(str::to_string)
}
