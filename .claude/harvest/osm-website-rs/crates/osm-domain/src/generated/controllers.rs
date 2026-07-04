//! @generated DO arm — a faithful `controllers` mirror. Each module is a
//! source controller by its own verbatim (snake) name; each fn is an `is_a`
//! action (standalone, not methods on the record). osm-domain re-exports this
//! module (`pub use controllers::*`), so `controllers::nodes::show(input)` and
//! the re-exported `nodes::show(input)` both resolve. No singularisation.

#![allow(clippy::all, dead_code, unused_variables)]

/// DO-arm action input — the Rails `params` / request. Typed-field harvest is a
/// follow-up (ruff `Function` carries reads/writes, not param types yet).
#[derive(Debug, Default)]
pub struct Input;

/// DO-arm action output — the Rails response.
#[derive(Debug, Default)]
pub struct Output;

pub mod accounts {
    use super::{Input, Output};
    /// `accounts:delete` — DO arm. Source: `AccountsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port AccountsController#destroy")
    }
    /// `accounts:show` — DO arm. Source: `AccountsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port AccountsController#show")
    }
    /// `accounts:update` — DO arm. Source: `AccountsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port AccountsController#update")
    }
}

pub mod active_lists {
    use super::{Input, Output};
    /// `active_lists:show` — DO arm. Source: `Api::UserBlocks::ActiveListsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserBlocks::ActiveListsController#show")
    }
}

pub mod advanced_preferences {
    use super::{Input, Output};
    /// `advanced_preferences:update_preferences` — DO arm. Source: `Preferences::AdvancedPreferencesController#update_preferences`.
    pub fn update_preferences(input: Input) -> Output {
        let _ = input;
        todo!("port Preferences::AdvancedPreferencesController#update_preferences")
    }
}

pub mod api {
    use super::{Input, Output};
    /// `api:api_call_handle_error` — DO arm. Source: `ApiController#api_call_handle_error`.
    pub fn api_call_handle_error(input: Input) -> Output {
        let _ = input;
        todo!("port ApiController#api_call_handle_error")
    }
    /// `api:api_call_timeout` — DO arm. Source: `ApiController#api_call_timeout`.
    pub fn api_call_timeout(input: Input) -> Output {
        let _ = input;
        todo!("port ApiController#api_call_timeout")
    }
    /// `api:authorize` — DO arm. Source: `ApiController#authorize`.
    pub fn authorize(input: Input) -> Output {
        let _ = input;
        todo!("port ApiController#authorize")
    }
    /// `api:check_rate_limit` — DO arm. Source: `ApiController#check_rate_limit`.
    pub fn check_rate_limit(input: Input) -> Output {
        let _ = input;
        todo!("port ApiController#check_rate_limit")
    }
    /// `api:current_ability` — DO arm. Source: `ApiController#current_ability`.
    pub fn current_ability(input: Input) -> Output {
        let _ = input;
        todo!("port ApiController#current_ability")
    }
    /// `api:deny_access` — DO arm. Source: `ApiController#deny_access`.
    pub fn deny_access(input: Input) -> Output {
        let _ = input;
        todo!("port ApiController#deny_access")
    }
    /// `api:gpx_status` — DO arm. Source: `ApiController#gpx_status`.
    pub fn gpx_status(input: Input) -> Output {
        let _ = input;
        todo!("port ApiController#gpx_status")
    }
    /// `api:scope_enabled?` — DO arm. Source: `ApiController#scope_enabled?`.
    pub fn scope_enabled(input: Input) -> Output {
        let _ = input;
        todo!("port ApiController#scope_enabled?")
    }
    /// `api:set_request_formats` — DO arm. Source: `ApiController#set_request_formats`.
    pub fn set_request_formats(input: Input) -> Output {
        let _ = input;
        todo!("port ApiController#set_request_formats")
    }
    /// `api:setup_user_auth` — DO arm. Source: `ApiController#setup_user_auth`.
    pub fn setup_user_auth(input: Input) -> Output {
        let _ = input;
        todo!("port ApiController#setup_user_auth")
    }
}

pub mod application {
    use super::{Input, Output};
    /// `application:api_status` — DO arm. Source: `ApplicationController#api_status`.
    pub fn api_status(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#api_status")
    }
    /// `application:authorize_web` — DO arm. Source: `ApplicationController#authorize_web`.
    pub fn authorize_web(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#authorize_web")
    }
    /// `application:check_api_readable` — DO arm. Source: `ApplicationController#check_api_readable`.
    pub fn check_api_readable(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#check_api_readable")
    }
    /// `application:check_api_writable` — DO arm. Source: `ApplicationController#check_api_writable`.
    pub fn check_api_writable(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#check_api_writable")
    }
    /// `application:check_database_readable` — DO arm. Source: `ApplicationController#check_database_readable`.
    pub fn check_database_readable(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#check_database_readable")
    }
    /// `application:check_database_writable` — DO arm. Source: `ApplicationController#check_database_writable`.
    pub fn check_database_writable(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#check_database_writable")
    }
    /// `application:close_body` — DO arm. Source: `ApplicationController#close_body`.
    pub fn close_body(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#close_body")
    }
    /// `application:current_ability` — DO arm. Source: `ApplicationController#current_ability`.
    pub fn current_ability(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#current_ability")
    }
    /// `application:database_status` — DO arm. Source: `ApplicationController#database_status`.
    pub fn database_status(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#database_status")
    }
    /// `application:deny_access` — DO arm. Source: `ApplicationController#deny_access`.
    pub fn deny_access(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#deny_access")
    }
    /// `application:invalid_parameter` — DO arm. Source: `ApplicationController#invalid_parameter`.
    pub fn invalid_parameter(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#invalid_parameter")
    }
    /// `application:map_layout` — DO arm. Source: `ApplicationController#map_layout`.
    pub fn map_layout(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#map_layout")
    }
    /// `application:preferred_editor` — DO arm. Source: `ApplicationController#preferred_editor`.
    pub fn preferred_editor(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#preferred_editor")
    }
    /// `application:preferred_languages` — DO arm. Source: `ApplicationController#preferred_languages`.
    pub fn preferred_languages(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#preferred_languages")
    }
    /// `application:report_error` — DO arm. Source: `ApplicationController#report_error`.
    pub fn report_error(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#report_error")
    }
    /// `application:require_cookies` — DO arm. Source: `ApplicationController#require_cookies`.
    pub fn require_cookies(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#require_cookies")
    }
    /// `application:require_oauth` — DO arm. Source: `ApplicationController#require_oauth`.
    pub fn require_oauth(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#require_oauth")
    }
    /// `application:require_public_data` — DO arm. Source: `ApplicationController#require_public_data`.
    pub fn require_public_data(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#require_public_data")
    }
    /// `application:require_user` — DO arm. Source: `ApplicationController#require_user`.
    pub fn require_user(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#require_user")
    }
    /// `application:respond_to_timeout` — DO arm. Source: `ApplicationController#respond_to_timeout`.
    pub fn respond_to_timeout(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#respond_to_timeout")
    }
    /// `application:safe_referer` — DO arm. Source: `ApplicationController#safe_referer`.
    pub fn safe_referer(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#safe_referer")
    }
    /// `application:set_locale` — DO arm. Source: `ApplicationController#set_locale`.
    pub fn set_locale(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#set_locale")
    }
    /// `application:site_layout` — DO arm. Source: `ApplicationController#site_layout`.
    pub fn site_layout(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#site_layout")
    }
    /// `application:update_totp` — DO arm. Source: `ApplicationController#update_totp`.
    pub fn update_totp(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#update_totp")
    }
    /// `application:web_timeout` — DO arm. Source: `ApplicationController#web_timeout`.
    pub fn web_timeout(input: Input) -> Output {
        let _ = input;
        todo!("port ApplicationController#web_timeout")
    }
}

pub mod basic_preferences {
    use super::{Input, Output};
    /// `basic_preferences:update_preferences` — DO arm. Source: `Preferences::BasicPreferencesController#update_preferences`.
    pub fn update_preferences(input: Input) -> Output {
        let _ = input;
        todo!("port Preferences::BasicPreferencesController#update_preferences")
    }
}

pub mod capabilities {
    use super::{Input, Output};
    /// `capabilities:show` — DO arm. Source: `Api::CapabilitiesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::CapabilitiesController#show")
    }
}

pub mod changeset_comments {
    use super::{Input, Output};
    /// `changeset_comments:create` — DO arm. Source: `Api::ChangesetCommentsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetCommentsController#create")
    }
    /// `changeset_comments:list` — DO arm. Sources (canonical tile): `Api::ChangesetCommentsController#index`, `Users::ChangesetCommentsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetCommentsController#index")
    }
    /// `changeset_comments:rate_limit_exceeded?` — DO arm. Source: `Api::ChangesetCommentsController#rate_limit_exceeded?`.
    pub fn rate_limit_exceeded(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetCommentsController#rate_limit_exceeded?")
    }
}

pub mod changeset_subscriptions {
    use super::{Input, Output};
    /// `changeset_subscriptions:create` — DO arm. Sources (canonical tile): `Api::ChangesetSubscriptionsController#create`, `ChangesetSubscriptionsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetSubscriptionsController#create")
    }
    /// `changeset_subscriptions:delete` — DO arm. Sources (canonical tile): `Api::ChangesetSubscriptionsController#destroy`, `ChangesetSubscriptionsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetSubscriptionsController#destroy")
    }
    /// `changeset_subscriptions:show` — DO arm. Source: `ChangesetSubscriptionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetSubscriptionsController#show")
    }
}

pub mod changesets {
    use super::{Input, Output};
    /// `changesets:conditions_bbox` — DO arm. Sources (canonical tile): `Api::ChangesetsController#conditions_bbox`, `ChangesetsController#conditions_bbox`.
    pub fn conditions_bbox(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_bbox")
    }
    /// `changesets:conditions_closed` — DO arm. Source: `Api::ChangesetsController#conditions_closed`.
    pub fn conditions_closed(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_closed")
    }
    /// `changesets:conditions_ids` — DO arm. Source: `Api::ChangesetsController#conditions_ids`.
    pub fn conditions_ids(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_ids")
    }
    /// `changesets:conditions_nonempty` — DO arm. Source: `ChangesetsController#conditions_nonempty`.
    pub fn conditions_nonempty(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#conditions_nonempty")
    }
    /// `changesets:conditions_open` — DO arm. Source: `Api::ChangesetsController#conditions_open`.
    pub fn conditions_open(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_open")
    }
    /// `changesets:conditions_time` — DO arm. Source: `Api::ChangesetsController#conditions_time`.
    pub fn conditions_time(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_time")
    }
    /// `changesets:conditions_user` — DO arm. Source: `Api::ChangesetsController#conditions_user`.
    pub fn conditions_user(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_user")
    }
    /// `changesets:create` — DO arm. Source: `Api::ChangesetsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#create")
    }
    /// `changesets:feed` — DO arm. Source: `ChangesetsController#feed`.
    pub fn feed(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#feed")
    }
    /// `changesets:list` — DO arm. Sources (canonical tile): `Api::ChangesetsController#index`, `ChangesetsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#index")
    }
    /// `changesets:load_nodes` — DO arm. Source: `ChangesetsController#load_nodes`.
    pub fn load_nodes(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#load_nodes")
    }
    /// `changesets:load_relations` — DO arm. Source: `ChangesetsController#load_relations`.
    pub fn load_relations(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#load_relations")
    }
    /// `changesets:load_ways` — DO arm. Source: `ChangesetsController#load_ways`.
    pub fn load_ways(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#load_ways")
    }
    /// `changesets:show` — DO arm. Sources (canonical tile): `Api::ChangesetsController#show`, `ChangesetsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#show")
    }
    /// `changesets:update` — DO arm. Source: `Api::ChangesetsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#update")
    }
    /// `changesets:wrap_lon` — DO arm. Source: `ChangesetsController#wrap_lon`.
    pub fn wrap_lon(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#wrap_lon")
    }
}

pub mod closes {
    use super::{Input, Output};
    /// `closes:update` — DO arm. Source: `Api::Changesets::ClosesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Changesets::ClosesController#update")
    }
}

pub mod companies {
    use super::{Input, Output};
    /// `companies:update_profile` — DO arm. Source: `Profiles::CompaniesController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::CompaniesController#update_profile")
    }
}

pub mod confirmations {
    use super::{Input, Output};
    /// `confirmations:confirm` — DO arm. Source: `ConfirmationsController#confirm`.
    pub fn confirm(input: Input) -> Output {
        let _ = input;
        todo!("port ConfirmationsController#confirm")
    }
    /// `confirmations:confirm_email` — DO arm. Source: `ConfirmationsController#confirm_email`.
    pub fn confirm_email(input: Input) -> Output {
        let _ = input;
        todo!("port ConfirmationsController#confirm_email")
    }
    /// `confirmations:confirm_resend` — DO arm. Source: `ConfirmationsController#confirm_resend`.
    pub fn confirm_resend(input: Input) -> Output {
        let _ = input;
        todo!("port ConfirmationsController#confirm_resend")
    }
    /// `confirmations:gravatar_status_message` — DO arm. Source: `ConfirmationsController#gravatar_status_message`.
    pub fn gravatar_status_message(input: Input) -> Output {
        let _ = input;
        todo!("port ConfirmationsController#gravatar_status_message")
    }
}

pub mod dashboards {
    use super::{Input, Output};
    /// `dashboards:show` — DO arm. Source: `DashboardsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port DashboardsController#show")
    }
}

pub mod data {
    use super::{Input, Output};
    /// `data:offline_error` — DO arm. Source: `Api::Traces::DataController#offline_error`.
    pub fn offline_error(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Traces::DataController#offline_error")
    }
    /// `data:offline_redirect` — DO arm. Source: `Traces::DataController#offline_redirect`.
    pub fn offline_redirect(input: Input) -> Output {
        let _ = input;
        todo!("port Traces::DataController#offline_redirect")
    }
    /// `data:show` — DO arm. Sources (canonical tile): `Api::Traces::DataController#show`, `Traces::DataController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Traces::DataController#show")
    }
}

pub mod deletions {
    use super::{Input, Output};
    /// `deletions:show` — DO arm. Source: `Accounts::DeletionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::DeletionsController#show")
    }
}

pub mod descriptions {
    use super::{Input, Output};
    /// `descriptions:update_profile` — DO arm. Source: `Profiles::DescriptionsController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::DescriptionsController#update_profile")
    }
}

pub mod diary_comments {
    use super::{Input, Output};
    /// `diary_comments:comment_params` — DO arm. Source: `DiaryCommentsController#comment_params`.
    pub fn comment_params(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryCommentsController#comment_params")
    }
    /// `diary_comments:create` — DO arm. Source: `DiaryCommentsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryCommentsController#create")
    }
    /// `diary_comments:hide` — DO arm. Source: `DiaryCommentsController#hide`.
    pub fn hide(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryCommentsController#hide")
    }
    /// `diary_comments:list` — DO arm. Source: `Users::DiaryCommentsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Users::DiaryCommentsController#index")
    }
    /// `diary_comments:unhide` — DO arm. Source: `DiaryCommentsController#unhide`.
    pub fn unhide(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryCommentsController#unhide")
    }
}

pub mod diary_entries {
    use super::{Input, Output};
    /// `diary_entries:create` — DO arm. Source: `DiaryEntriesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#create")
    }
    /// `diary_entries:edit` — DO arm. Source: `DiaryEntriesController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#edit")
    }
    /// `diary_entries:entry_params` — DO arm. Source: `DiaryEntriesController#entry_params`.
    pub fn entry_params(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#entry_params")
    }
    /// `diary_entries:hide` — DO arm. Source: `DiaryEntriesController#hide`.
    pub fn hide(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#hide")
    }
    /// `diary_entries:list` — DO arm. Source: `DiaryEntriesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#index")
    }
    /// `diary_entries:new_form` — DO arm. Source: `DiaryEntriesController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#new")
    }
    /// `diary_entries:rss` — DO arm. Source: `DiaryEntriesController#rss`.
    pub fn rss(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#rss")
    }
    /// `diary_entries:set_map_location` — DO arm. Source: `DiaryEntriesController#set_map_location`.
    pub fn set_map_location(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#set_map_location")
    }
    /// `diary_entries:show` — DO arm. Source: `DiaryEntriesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#show")
    }
    /// `diary_entries:subscribe` — DO arm. Source: `DiaryEntriesController#subscribe`.
    pub fn subscribe(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#subscribe")
    }
    /// `diary_entries:unhide` — DO arm. Source: `DiaryEntriesController#unhide`.
    pub fn unhide(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#unhide")
    }
    /// `diary_entries:unsubscribe` — DO arm. Source: `DiaryEntriesController#unsubscribe`.
    pub fn unsubscribe(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#unsubscribe")
    }
    /// `diary_entries:update` — DO arm. Source: `DiaryEntriesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#update")
    }
}

pub mod directions {
    use super::{Input, Output};
    /// `directions:show` — DO arm. Source: `DirectionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port DirectionsController#show")
    }
}

pub mod downloads {
    use super::{Input, Output};
    /// `downloads:show` — DO arm. Source: `Api::Changesets::DownloadsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Changesets::DownloadsController#show")
    }
    /// `downloads:show_redactions?` — DO arm. Source: `Api::Changesets::DownloadsController#show_redactions?`.
    pub fn show_redactions(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Changesets::DownloadsController#show_redactions?")
    }
}

pub mod errors {
    use super::{Input, Output};
    /// `errors:bad_request` — DO arm. Source: `ErrorsController#bad_request`.
    pub fn bad_request(input: Input) -> Output {
        let _ = input;
        todo!("port ErrorsController#bad_request")
    }
    /// `errors:forbidden` — DO arm. Source: `ErrorsController#forbidden`.
    pub fn forbidden(input: Input) -> Output {
        let _ = input;
        todo!("port ErrorsController#forbidden")
    }
    /// `errors:internal_server_error` — DO arm. Source: `ErrorsController#internal_server_error`.
    pub fn internal_server_error(input: Input) -> Output {
        let _ = input;
        todo!("port ErrorsController#internal_server_error")
    }
    /// `errors:not_found` — DO arm. Source: `ErrorsController#not_found`.
    pub fn not_found(input: Input) -> Output {
        let _ = input;
        todo!("port ErrorsController#not_found")
    }
}

pub mod export {
    use super::{Input, Output};
    /// `export:create` — DO arm. Source: `ExportController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port ExportController#create")
    }
    /// `export:show` — DO arm. Source: `ExportController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port ExportController#show")
    }
}

pub mod feature_queries {
    use super::{Input, Output};
    /// `feature_queries:show` — DO arm. Source: `FeatureQueriesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port FeatureQueriesController#show")
    }
}

pub mod feeds {
    use super::{Input, Output};
    /// `feeds:show` — DO arm. Sources (canonical tile): `ChangesetComments::FeedsController#show`, `Traces::FeedsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetComments::FeedsController#show")
    }
}

pub mod follows {
    use super::{Input, Output};
    /// `follows:create` — DO arm. Source: `FollowsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port FollowsController#create")
    }
    /// `follows:delete` — DO arm. Source: `FollowsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port FollowsController#destroy")
    }
    /// `follows:show` — DO arm. Source: `FollowsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port FollowsController#show")
    }
}

pub mod heatmaps {
    use super::{Input, Output};
    /// `heatmaps:show` — DO arm. Source: `Users::HeatmapsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Users::HeatmapsController#show")
    }
    /// `heatmaps:update_profile` — DO arm. Source: `Profiles::HeatmapsController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::HeatmapsController#update_profile")
    }
}

pub mod homes {
    use super::{Input, Output};
    /// `homes:show` — DO arm. Source: `Accounts::HomesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::HomesController#show")
    }
}

pub mod icons {
    use super::{Input, Output};
    /// `icons:show` — DO arm. Source: `Traces::IconsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Traces::IconsController#show")
    }
}

pub mod images {
    use super::{Input, Output};
    /// `images:update_profile` — DO arm. Source: `Profiles::ImagesController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::ImagesController#update_profile")
    }
}

pub mod inboxes {
    use super::{Input, Output};
    /// `inboxes:show` — DO arm. Sources (canonical tile): `Api::Messages::InboxesController#show`, `Messages::InboxesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Messages::InboxesController#show")
    }
}

pub mod issue_comments {
    use super::{Input, Output};
    /// `issue_comments:create` — DO arm. Source: `IssueCommentsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port IssueCommentsController#create")
    }
    /// `issue_comments:issue_comment_params` — DO arm. Source: `IssueCommentsController#issue_comment_params`.
    pub fn issue_comment_params(input: Input) -> Output {
        let _ = input;
        todo!("port IssueCommentsController#issue_comment_params")
    }
    /// `issue_comments:reassign_issue` — DO arm. Source: `IssueCommentsController#reassign_issue`.
    pub fn reassign_issue(input: Input) -> Output {
        let _ = input;
        todo!("port IssueCommentsController#reassign_issue")
    }
}

pub mod issued_blocks {
    use super::{Input, Output};
    /// `issued_blocks:show` — DO arm. Source: `Users::IssuedBlocksController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Users::IssuedBlocksController#show")
    }
}

pub mod issues {
    use super::{Input, Output};
    /// `issues:find_issue` — DO arm. Source: `IssuesController#find_issue`.
    pub fn find_issue(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#find_issue")
    }
    /// `issues:ignore` — DO arm. Source: `IssuesController#ignore`.
    pub fn ignore(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#ignore")
    }
    /// `issues:list` — DO arm. Source: `IssuesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#index")
    }
    /// `issues:reopen` — DO arm. Source: `IssuesController#reopen`.
    pub fn reopen(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#reopen")
    }
    /// `issues:resolve` — DO arm. Source: `IssuesController#resolve`.
    pub fn resolve(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#resolve")
    }
    /// `issues:show` — DO arm. Source: `IssuesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#show")
    }
}

pub mod languages_panes {
    use super::{Input, Output};
    /// `languages_panes:show` — DO arm. Source: `LanguagesPanesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port LanguagesPanesController#show")
    }
}

pub mod latlon_queries {
    use super::{Input, Output};
    /// `latlon_queries:create` — DO arm. Source: `Searches::LatlonQueriesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Searches::LatlonQueriesController#create")
    }
}

pub mod layers_panes {
    use super::{Input, Output};
    /// `layers_panes:show` — DO arm. Source: `LayersPanesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port LayersPanesController#show")
    }
}

pub mod legend_panes {
    use super::{Input, Output};
    /// `legend_panes:show` — DO arm. Source: `LegendPanesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port LegendPanesController#show")
    }
}

pub mod links {
    use super::{Input, Output};
    /// `links:update_profile` — DO arm. Source: `Profiles::LinksController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::LinksController#update_profile")
    }
}

pub mod lists {
    use super::{Input, Output};
    /// `lists:show` — DO arm. Source: `Users::ListsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Users::ListsController#show")
    }
    /// `lists:update` — DO arm. Source: `Users::ListsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Users::ListsController#update")
    }
}

pub mod locations {
    use super::{Input, Output};
    /// `locations:show` — DO arm. Source: `Profiles::LocationsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::LocationsController#show")
    }
    /// `locations:update_profile` — DO arm. Source: `Profiles::LocationsController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::LocationsController#update_profile")
    }
}

pub mod mailboxes {
    use super::{Input, Output};
    /// `mailboxes:show_messages` — DO arm. Source: `Api::Messages::MailboxesController#show_messages`.
    pub fn show_messages(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Messages::MailboxesController#show_messages")
    }
}

pub mod maps {
    use super::{Input, Output};
    /// `maps:show` — DO arm. Source: `Api::MapsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::MapsController#show")
    }
}

pub mod messages {
    use super::{Input, Output};
    /// `messages:create` — DO arm. Sources (canonical tile): `Api::MessagesController#create`, `MessagesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::MessagesController#create")
    }
    /// `messages:delete` — DO arm. Sources (canonical tile): `Api::MessagesController#destroy`, `MessagesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::MessagesController#destroy")
    }
    /// `messages:message_params` — DO arm. Source: `MessagesController#message_params`.
    pub fn message_params(input: Input) -> Output {
        let _ = input;
        todo!("port MessagesController#message_params")
    }
    /// `messages:new_form` — DO arm. Source: `MessagesController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port MessagesController#new")
    }
    /// `messages:show` — DO arm. Sources (canonical tile): `Api::MessagesController#show`, `MessagesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::MessagesController#show")
    }
    /// `messages:update` — DO arm. Source: `Api::MessagesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::MessagesController#update")
    }
}

pub mod muted_inboxes {
    use super::{Input, Output};
    /// `muted_inboxes:show` — DO arm. Source: `Messages::MutedInboxesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::MutedInboxesController#show")
    }
}

pub mod mutes {
    use super::{Input, Output};
    /// `mutes:delete` — DO arm. Source: `Messages::MutesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::MutesController#destroy")
    }
}

pub mod nodes {
    use super::{Input, Output};
    /// `nodes:create` — DO arm. Source: `Api::NodesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NodesController#create")
    }
    /// `nodes:delete` — DO arm. Source: `Api::NodesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NodesController#destroy")
    }
    /// `nodes:list` — DO arm. Source: `Api::NodesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NodesController#index")
    }
    /// `nodes:show` — DO arm. Sources (canonical tile): `Api::NodesController#show`, `NodesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NodesController#show")
    }
    /// `nodes:update` — DO arm. Source: `Api::NodesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NodesController#update")
    }
}

pub mod nominatim_queries {
    use super::{Input, Output};
    /// `nominatim_queries:create` — DO arm. Source: `Searches::NominatimQueriesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Searches::NominatimQueriesController#create")
    }
}

pub mod nominatim_reverse_queries {
    use super::{Input, Output};
    /// `nominatim_reverse_queries:create` — DO arm. Source: `Searches::NominatimReverseQueriesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Searches::NominatimReverseQueriesController#create")
    }
}

pub mod note_subscriptions {
    use super::{Input, Output};
    /// `note_subscriptions:create` — DO arm. Source: `Api::NoteSubscriptionsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NoteSubscriptionsController#create")
    }
    /// `note_subscriptions:delete` — DO arm. Source: `Api::NoteSubscriptionsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NoteSubscriptionsController#destroy")
    }
}

pub mod notes {
    use super::{Input, Output};
    /// `notes:add_comment` — DO arm. Source: `Api::NotesController#add_comment`.
    pub fn add_comment(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#add_comment")
    }
    /// `notes:author_info` — DO arm. Source: `Api::NotesController#author_info`.
    pub fn author_info(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#author_info")
    }
    /// `notes:bbox_condition` — DO arm. Source: `Api::NotesController#bbox_condition`.
    pub fn bbox_condition(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#bbox_condition")
    }
    /// `notes:close` — DO arm. Source: `Api::NotesController#close`.
    pub fn close(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#close")
    }
    /// `notes:closed_condition` — DO arm. Source: `Api::NotesController#closed_condition`.
    pub fn closed_condition(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#closed_condition")
    }
    /// `notes:comment` — DO arm. Source: `Api::NotesController#comment`.
    pub fn comment(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#comment")
    }
    /// `notes:create` — DO arm. Source: `Api::NotesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#create")
    }
    /// `notes:delete` — DO arm. Source: `Api::NotesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#destroy")
    }
    /// `notes:feed` — DO arm. Source: `Api::NotesController#feed`.
    pub fn feed(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#feed")
    }
    /// `notes:list` — DO arm. Sources (canonical tile): `Api::NotesController#index`, `NotesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#index")
    }
    /// `notes:new_form` — DO arm. Source: `NotesController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port NotesController#new")
    }
    /// `notes:reopen` — DO arm. Source: `Api::NotesController#reopen`.
    pub fn reopen(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#reopen")
    }
    /// `notes:search` — DO arm. Source: `Api::NotesController#search`.
    pub fn search(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#search")
    }
    /// `notes:show` — DO arm. Sources (canonical tile): `Api::NotesController#show`, `NotesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#show")
    }
}

pub mod notification_preferences {
    use super::{Input, Output};
    /// `notification_preferences:update_preferences` — DO arm. Source: `Preferences::NotificationPreferencesController#update_preferences`.
    pub fn update_preferences(input: Input) -> Output {
        let _ = input;
        todo!("port Preferences::NotificationPreferencesController#update_preferences")
    }
}

pub mod oauth2_applications {
    use super::{Input, Output};
    /// `oauth2_applications:application_params` — DO arm. Source: `Oauth2ApplicationsController#application_params`.
    pub fn application_params(input: Input) -> Output {
        let _ = input;
        todo!("port Oauth2ApplicationsController#application_params")
    }
    /// `oauth2_applications:list` — DO arm. Source: `Oauth2ApplicationsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Oauth2ApplicationsController#index")
    }
    /// `oauth2_applications:set_application` — DO arm. Source: `Oauth2ApplicationsController#set_application`.
    pub fn set_application(input: Input) -> Output {
        let _ = input;
        todo!("port Oauth2ApplicationsController#set_application")
    }
}

pub mod old_elements {
    use super::{Input, Output};
    /// `old_elements:list` — DO arm. Source: `Api::OldElementsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldElementsController#index")
    }
    /// `old_elements:require_moderator_for_unredacted_history` — DO arm. Source: `OldElementsController#require_moderator_for_unredacted_history`.
    pub fn require_moderator_for_unredacted_history(input: Input) -> Output {
        let _ = input;
        todo!("port OldElementsController#require_moderator_for_unredacted_history")
    }
    /// `old_elements:show` — DO arm. Source: `Api::OldElementsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldElementsController#show")
    }
    /// `old_elements:show_redactions?` — DO arm. Source: `Api::OldElementsController#show_redactions?`.
    pub fn show_redactions(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldElementsController#show_redactions?")
    }
}

pub mod old_nodes {
    use super::{Input, Output};
    /// `old_nodes:list` — DO arm. Source: `OldNodesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port OldNodesController#index")
    }
    /// `old_nodes:lookup_old_element` — DO arm. Source: `Api::OldNodesController#lookup_old_element`.
    pub fn lookup_old_element(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldNodesController#lookup_old_element")
    }
    /// `old_nodes:lookup_old_element_versions` — DO arm. Source: `Api::OldNodesController#lookup_old_element_versions`.
    pub fn lookup_old_element_versions(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldNodesController#lookup_old_element_versions")
    }
    /// `old_nodes:show` — DO arm. Source: `OldNodesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port OldNodesController#show")
    }
}

pub mod old_relation_members {
    use super::{Input, Output};
    /// `old_relation_members:show` — DO arm. Source: `OldRelationMembersController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port OldRelationMembersController#show")
    }
}

pub mod old_relations {
    use super::{Input, Output};
    /// `old_relations:list` — DO arm. Source: `OldRelationsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port OldRelationsController#index")
    }
    /// `old_relations:lookup_old_element` — DO arm. Source: `Api::OldRelationsController#lookup_old_element`.
    pub fn lookup_old_element(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldRelationsController#lookup_old_element")
    }
    /// `old_relations:lookup_old_element_versions` — DO arm. Source: `Api::OldRelationsController#lookup_old_element_versions`.
    pub fn lookup_old_element_versions(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldRelationsController#lookup_old_element_versions")
    }
    /// `old_relations:show` — DO arm. Source: `OldRelationsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port OldRelationsController#show")
    }
}

pub mod old_ways {
    use super::{Input, Output};
    /// `old_ways:list` — DO arm. Source: `OldWaysController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port OldWaysController#index")
    }
    /// `old_ways:lookup_old_element` — DO arm. Source: `Api::OldWaysController#lookup_old_element`.
    pub fn lookup_old_element(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldWaysController#lookup_old_element")
    }
    /// `old_ways:lookup_old_element_versions` — DO arm. Source: `Api::OldWaysController#lookup_old_element_versions`.
    pub fn lookup_old_element_versions(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldWaysController#lookup_old_element_versions")
    }
    /// `old_ways:show` — DO arm. Source: `OldWaysController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port OldWaysController#show")
    }
}

pub mod outboxes {
    use super::{Input, Output};
    /// `outboxes:show` — DO arm. Sources (canonical tile): `Api::Messages::OutboxesController#show`, `Messages::OutboxesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Messages::OutboxesController#show")
    }
}

pub mod passwords {
    use super::{Input, Output};
    /// `passwords:create` — DO arm. Source: `PasswordsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port PasswordsController#create")
    }
    /// `passwords:edit` — DO arm. Source: `PasswordsController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port PasswordsController#edit")
    }
    /// `passwords:new_form` — DO arm. Source: `PasswordsController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port PasswordsController#new")
    }
    /// `passwords:update` — DO arm. Source: `PasswordsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port PasswordsController#update")
    }
}

pub mod pd_declarations {
    use super::{Input, Output};
    /// `pd_declarations:create` — DO arm. Source: `Accounts::PdDeclarationsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::PdDeclarationsController#create")
    }
    /// `pd_declarations:show` — DO arm. Source: `Accounts::PdDeclarationsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::PdDeclarationsController#show")
    }
}

pub mod permissions {
    use super::{Input, Output};
    /// `permissions:show` — DO arm. Source: `Api::PermissionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::PermissionsController#show")
    }
}

pub mod pictures {
    use super::{Input, Output};
    /// `pictures:show` — DO arm. Source: `Traces::PicturesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Traces::PicturesController#show")
    }
}

pub mod preferences {
    use super::{Input, Output};
    /// `preferences:show` — DO arm. Source: `Preferences::PreferencesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Preferences::PreferencesController#show")
    }
    /// `preferences:update` — DO arm. Source: `Preferences::PreferencesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Preferences::PreferencesController#update")
    }
}

pub mod profile_sections {
    use super::{Input, Output};
    /// `profile_sections:show` — DO arm. Source: `Profiles::ProfileSectionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::ProfileSectionsController#show")
    }
    /// `profile_sections:update` — DO arm. Source: `Profiles::ProfileSectionsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::ProfileSectionsController#update")
    }
}

pub mod queries {
    use super::{Input, Output};
    /// `queries:fetch_text` — DO arm. Source: `Searches::QueriesController#fetch_text`.
    pub fn fetch_text(input: Input) -> Output {
        let _ = input;
        todo!("port Searches::QueriesController#fetch_text")
    }
    /// `queries:fetch_xml` — DO arm. Source: `Searches::QueriesController#fetch_xml`.
    pub fn fetch_xml(input: Input) -> Output {
        let _ = input;
        todo!("port Searches::QueriesController#fetch_xml")
    }
}

pub mod read_marks {
    use super::{Input, Output};
    /// `read_marks:create` — DO arm. Source: `Messages::ReadMarksController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::ReadMarksController#create")
    }
    /// `read_marks:delete` — DO arm. Source: `Messages::ReadMarksController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::ReadMarksController#destroy")
    }
    /// `read_marks:mark` — DO arm. Source: `Messages::ReadMarksController#mark`.
    pub fn mark(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::ReadMarksController#mark")
    }
}

pub mod received_blocks {
    use super::{Input, Output};
    /// `received_blocks:delete` — DO arm. Source: `Users::ReceivedBlocksController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Users::ReceivedBlocksController#destroy")
    }
    /// `received_blocks:edit` — DO arm. Source: `Users::ReceivedBlocksController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port Users::ReceivedBlocksController#edit")
    }
    /// `received_blocks:show` — DO arm. Source: `Users::ReceivedBlocksController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Users::ReceivedBlocksController#show")
    }
}

pub mod redactions {
    use super::{Input, Output};
    /// `redactions:create` — DO arm. Sources (canonical tile): `Api::OldElements::RedactionsController#create`, `RedactionsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldElements::RedactionsController#create")
    }
    /// `redactions:delete` — DO arm. Sources (canonical tile): `Api::OldElements::RedactionsController#destroy`, `RedactionsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldElements::RedactionsController#destroy")
    }
    /// `redactions:edit` — DO arm. Source: `RedactionsController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#edit")
    }
    /// `redactions:list` — DO arm. Source: `RedactionsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#index")
    }
    /// `redactions:lookup_old_element` — DO arm. Sources (canonical tile): `Api::OldNodes::RedactionsController#lookup_old_element`, `Api::OldRelations::RedactionsController#lookup_old_element`, `Api::OldWays::RedactionsController#lookup_old_element`.
    pub fn lookup_old_element(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldNodes::RedactionsController#lookup_old_element")
    }
    /// `redactions:lookup_redaction` — DO arm. Source: `RedactionsController#lookup_redaction`.
    pub fn lookup_redaction(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#lookup_redaction")
    }
    /// `redactions:new_form` — DO arm. Source: `RedactionsController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#new")
    }
    /// `redactions:show` — DO arm. Source: `RedactionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#show")
    }
    /// `redactions:update` — DO arm. Source: `RedactionsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#update")
    }
}

pub mod relation_members {
    use super::{Input, Output};
    /// `relation_members:show` — DO arm. Source: `RelationMembersController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port RelationMembersController#show")
    }
}

pub mod relations {
    use super::{Input, Output};
    /// `relations:create` — DO arm. Source: `Api::RelationsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::RelationsController#create")
    }
    /// `relations:delete` — DO arm. Source: `Api::RelationsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::RelationsController#destroy")
    }
    /// `relations:list` — DO arm. Sources (canonical tile): `Api::Nodes::RelationsController#index`, `Api::Relations::RelationsController#index`, `Api::RelationsController#index`, `Api::Ways::RelationsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Nodes::RelationsController#index")
    }
    /// `relations:show` — DO arm. Sources (canonical tile): `Api::RelationsController#show`, `RelationsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::RelationsController#show")
    }
    /// `relations:update` — DO arm. Source: `Api::RelationsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::RelationsController#update")
    }
}

pub mod replies {
    use super::{Input, Output};
    /// `replies:new_form` — DO arm. Source: `Messages::RepliesController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::RepliesController#new")
    }
}

pub mod reporters {
    use super::{Input, Output};
    /// `reporters:list` — DO arm. Source: `Issues::ReportersController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Issues::ReportersController#index")
    }
}

pub mod reports {
    use super::{Input, Output};
    /// `reports:create` — DO arm. Source: `ReportsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#create")
    }
    /// `reports:create_new_report_params` — DO arm. Source: `ReportsController#create_new_report_params`.
    pub fn create_new_report_params(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#create_new_report_params")
    }
    /// `reports:default_assigned_role` — DO arm. Source: `ReportsController#default_assigned_role`.
    pub fn default_assigned_role(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#default_assigned_role")
    }
    /// `reports:issue_params` — DO arm. Source: `ReportsController#issue_params`.
    pub fn issue_params(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#issue_params")
    }
    /// `reports:new_form` — DO arm. Source: `ReportsController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#new")
    }
    /// `reports:report_params` — DO arm. Source: `ReportsController#report_params`.
    pub fn report_params(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#report_params")
    }
    /// `reports:required_new_report_params_present?` — DO arm. Source: `ReportsController#required_new_report_params_present?`.
    pub fn required_new_report_params_present(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#required_new_report_params_present?")
    }
}

pub mod searches {
    use super::{Input, Output};
    /// `searches:dms_regexp` — DO arm. Source: `SearchesController#dms_regexp`.
    pub fn dms_regexp(input: Input) -> Output {
        let _ = input;
        todo!("port SearchesController#dms_regexp")
    }
    /// `searches:normalize_params` — DO arm. Source: `SearchesController#normalize_params`.
    pub fn normalize_params(input: Input) -> Output {
        let _ = input;
        todo!("port SearchesController#normalize_params")
    }
    /// `searches:show` — DO arm. Source: `SearchesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port SearchesController#show")
    }
    /// `searches:to_decdeg` — DO arm. Source: `SearchesController#to_decdeg`.
    pub fn to_decdeg(input: Input) -> Output {
        let _ = input;
        todo!("port SearchesController#to_decdeg")
    }
}

pub mod sessions {
    use super::{Input, Output};
    /// `sessions:create` — DO arm. Source: `SessionsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port SessionsController#create")
    }
    /// `sessions:delete` — DO arm. Source: `SessionsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port SessionsController#destroy")
    }
    /// `sessions:new_form` — DO arm. Source: `SessionsController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port SessionsController#new")
    }
    /// `sessions:password_authentication` — DO arm. Source: `SessionsController#password_authentication`.
    pub fn password_authentication(input: Input) -> Output {
        let _ = input;
        todo!("port SessionsController#password_authentication")
    }
}

pub mod share_panes {
    use super::{Input, Output};
    /// `share_panes:show` — DO arm. Source: `SharePanesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port SharePanesController#show")
    }
}

pub mod site {
    use super::{Input, Output};
    /// `site:about` — DO arm. Source: `SiteController#about`.
    pub fn about(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#about")
    }
    /// `site:communities` — DO arm. Source: `SiteController#communities`.
    pub fn communities(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#communities")
    }
    /// `site:copyright` — DO arm. Source: `SiteController#copyright`.
    pub fn copyright(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#copyright")
    }
    /// `site:edit` — DO arm. Source: `SiteController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#edit")
    }
    /// `site:export` — DO arm. Source: `SiteController#export`.
    pub fn export(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#export")
    }
    /// `site:help` — DO arm. Source: `SiteController#help`.
    pub fn help(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#help")
    }
    /// `site:id` — DO arm. Source: `SiteController#id`.
    pub fn id(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#id")
    }
    /// `site:list` — DO arm. Source: `SiteController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#index")
    }
    /// `site:offline` — DO arm. Source: `SiteController#offline`.
    pub fn offline(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#offline")
    }
    /// `site:permalink` — DO arm. Source: `SiteController#permalink`.
    pub fn permalink(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#permalink")
    }
    /// `site:preview` — DO arm. Source: `SiteController#preview`.
    pub fn preview(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#preview")
    }
    /// `site:redirect_browse_params` — DO arm. Source: `SiteController#redirect_browse_params`.
    pub fn redirect_browse_params(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#redirect_browse_params")
    }
    /// `site:redirect_map_params` — DO arm. Source: `SiteController#redirect_map_params`.
    pub fn redirect_map_params(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#redirect_map_params")
    }
    /// `site:welcome` — DO arm. Source: `SiteController#welcome`.
    pub fn welcome(input: Input) -> Output {
        let _ = input;
        todo!("port SiteController#welcome")
    }
}

pub mod statuses {
    use super::{Input, Output};
    /// `statuses:lookup_user_by_name` — DO arm. Source: `Users::StatusesController#lookup_user_by_name`.
    pub fn lookup_user_by_name(input: Input) -> Output {
        let _ = input;
        todo!("port Users::StatusesController#lookup_user_by_name")
    }
    /// `statuses:update` — DO arm. Source: `Users::StatusesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Users::StatusesController#update")
    }
}

pub mod terms {
    use super::{Input, Output};
    /// `terms:show` — DO arm. Source: `Accounts::TermsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::TermsController#show")
    }
    /// `terms:update` — DO arm. Source: `Accounts::TermsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::TermsController#update")
    }
}

pub mod tracepoints {
    use super::{Input, Output};
    /// `tracepoints:list` — DO arm. Source: `Api::TracepointsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracepointsController#index")
    }
}

pub mod traces {
    use super::{Input, Output};
    /// `traces:create` — DO arm. Sources (canonical tile): `Api::TracesController#create`, `TracesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#create")
    }
    /// `traces:default_visibility` — DO arm. Source: `TracesController#default_visibility`.
    pub fn default_visibility(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#default_visibility")
    }
    /// `traces:delete` — DO arm. Sources (canonical tile): `Api::TracesController#destroy`, `TracesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#destroy")
    }
    /// `traces:do_create` — DO arm. Sources (canonical tile): `Api::TracesController#do_create`, `TracesController#do_create`.
    pub fn do_create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#do_create")
    }
    /// `traces:edit` — DO arm. Source: `TracesController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#edit")
    }
    /// `traces:list` — DO arm. Sources (canonical tile): `Api::Users::TracesController#index`, `TracesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Users::TracesController#index")
    }
    /// `traces:mine` — DO arm. Source: `TracesController#mine`.
    pub fn mine(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#mine")
    }
    /// `traces:new_form` — DO arm. Source: `TracesController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#new")
    }
    /// `traces:offline_error` — DO arm. Source: `Api::TracesController#offline_error`.
    pub fn offline_error(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#offline_error")
    }
    /// `traces:offline_redirect` — DO arm. Source: `TracesController#offline_redirect`.
    pub fn offline_redirect(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#offline_redirect")
    }
    /// `traces:offline_warning` — DO arm. Source: `TracesController#offline_warning`.
    pub fn offline_warning(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#offline_warning")
    }
    /// `traces:show` — DO arm. Sources (canonical tile): `Api::TracesController#show`, `TracesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#show")
    }
    /// `traces:trace_params` — DO arm. Source: `TracesController#trace_params`.
    pub fn trace_params(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#trace_params")
    }
    /// `traces:update` — DO arm. Sources (canonical tile): `Api::TracesController#update`, `TracesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#update")
    }
}

pub mod uploads {
    use super::{Input, Output};
    /// `uploads:create` — DO arm. Source: `Api::Changesets::UploadsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Changesets::UploadsController#create")
    }
}

pub mod user_blocks {
    use super::{Input, Output};
    /// `user_blocks:create` — DO arm. Sources (canonical tile): `Api::UserBlocksController#create`, `UserBlocksController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserBlocksController#create")
    }
    /// `user_blocks:edit` — DO arm. Source: `UserBlocksController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#edit")
    }
    /// `user_blocks:list` — DO arm. Source: `UserBlocksController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#index")
    }
    /// `user_blocks:lookup_user_block` — DO arm. Source: `UserBlocksController#lookup_user_block`.
    pub fn lookup_user_block(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#lookup_user_block")
    }
    /// `user_blocks:new_form` — DO arm. Source: `UserBlocksController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#new")
    }
    /// `user_blocks:require_valid_params` — DO arm. Source: `UserBlocksController#require_valid_params`.
    pub fn require_valid_params(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#require_valid_params")
    }
    /// `user_blocks:show` — DO arm. Sources (canonical tile): `Api::UserBlocksController#show`, `UserBlocksController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserBlocksController#show")
    }
    /// `user_blocks:update` — DO arm. Source: `UserBlocksController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#update")
    }
}

pub mod user_mutes {
    use super::{Input, Output};
    /// `user_mutes:create` — DO arm. Source: `UserMutesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port UserMutesController#create")
    }
    /// `user_mutes:delete` — DO arm. Source: `UserMutesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port UserMutesController#destroy")
    }
    /// `user_mutes:list` — DO arm. Source: `UserMutesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port UserMutesController#index")
    }
}

pub mod user_preferences {
    use super::{Input, Output};
    /// `user_preferences:delete` — DO arm. Source: `Api::UserPreferencesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserPreferencesController#destroy")
    }
    /// `user_preferences:list` — DO arm. Source: `Api::UserPreferencesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserPreferencesController#index")
    }
    /// `user_preferences:show` — DO arm. Source: `Api::UserPreferencesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserPreferencesController#show")
    }
    /// `user_preferences:update` — DO arm. Source: `Api::UserPreferencesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserPreferencesController#update")
    }
    /// `user_preferences:update_all` — DO arm. Source: `Api::UserPreferencesController#update_all`.
    pub fn update_all(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserPreferencesController#update_all")
    }
}

pub mod user_roles {
    use super::{Input, Output};
    /// `user_roles:create` — DO arm. Source: `UserRolesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port UserRolesController#create")
    }
    /// `user_roles:delete` — DO arm. Source: `UserRolesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port UserRolesController#destroy")
    }
    /// `user_roles:in_role` — DO arm. Source: `UserRolesController#in_role`.
    pub fn in_role(input: Input) -> Output {
        let _ = input;
        todo!("port UserRolesController#in_role")
    }
    /// `user_roles:not_in_role` — DO arm. Source: `UserRolesController#not_in_role`.
    pub fn not_in_role(input: Input) -> Output {
        let _ = input;
        todo!("port UserRolesController#not_in_role")
    }
    /// `user_roles:require_valid_role` — DO arm. Source: `UserRolesController#require_valid_role`.
    pub fn require_valid_role(input: Input) -> Output {
        let _ = input;
        todo!("port UserRolesController#require_valid_role")
    }
}

pub mod users {
    use super::{Input, Output};
    /// `users:auth_failure` — DO arm. Source: `UsersController#auth_failure`.
    pub fn auth_failure(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#auth_failure")
    }
    /// `users:auth_success` — DO arm. Source: `UsersController#auth_success`.
    pub fn auth_success(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#auth_success")
    }
    /// `users:check_signup_allowed?` — DO arm. Source: `UsersController#check_signup_allowed?`.
    pub fn check_signup_allowed(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#check_signup_allowed?")
    }
    /// `users:create` — DO arm. Source: `UsersController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#create")
    }
    /// `users:details` — DO arm. Source: `Api::UsersController#details`.
    pub fn details(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UsersController#details")
    }
    /// `users:go_public` — DO arm. Source: `UsersController#go_public`.
    pub fn go_public(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#go_public")
    }
    /// `users:list` — DO arm. Source: `Api::UsersController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UsersController#index")
    }
    /// `users:new_form` — DO arm. Source: `UsersController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#new")
    }
    /// `users:save_new_user` — DO arm. Source: `UsersController#save_new_user`.
    pub fn save_new_user(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#save_new_user")
    }
    /// `users:show` — DO arm. Sources (canonical tile): `Api::UsersController#show`, `UsersController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UsersController#show")
    }
    /// `users:user_params` — DO arm. Source: `UsersController#user_params`.
    pub fn user_params(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#user_params")
    }
    /// `users:valid_turnstile_response?` — DO arm. Source: `UsersController#valid_turnstile_response?`.
    pub fn valid_turnstile_response(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#valid_turnstile_response?")
    }
    /// `users:welcome_options` — DO arm. Source: `UsersController#welcome_options`.
    pub fn welcome_options(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#welcome_options")
    }
}

pub mod versions {
    use super::{Input, Output};
    /// `versions:show` — DO arm. Source: `Api::VersionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::VersionsController#show")
    }
}

pub mod visibilities {
    use super::{Input, Output};
    /// `visibilities:create` — DO arm. Source: `Api::ChangesetComments::VisibilitiesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetComments::VisibilitiesController#create")
    }
    /// `visibilities:delete` — DO arm. Source: `Api::ChangesetComments::VisibilitiesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetComments::VisibilitiesController#destroy")
    }
}

pub mod ways {
    use super::{Input, Output};
    /// `ways:create` — DO arm. Source: `Api::WaysController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::WaysController#create")
    }
    /// `ways:delete` — DO arm. Source: `Api::WaysController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::WaysController#destroy")
    }
    /// `ways:list` — DO arm. Sources (canonical tile): `Api::Nodes::WaysController#index`, `Api::WaysController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Nodes::WaysController#index")
    }
    /// `ways:show` — DO arm. Sources (canonical tile): `Api::WaysController#show`, `WaysController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::WaysController#show")
    }
    /// `ways:update` — DO arm. Source: `Api::WaysController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::WaysController#update")
    }
}

pub mod webgl_error_panes {
    use super::{Input, Output};
    /// `webgl_error_panes:show` — DO arm. Source: `WebglErrorPanesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port WebglErrorPanesController#show")
    }
}

