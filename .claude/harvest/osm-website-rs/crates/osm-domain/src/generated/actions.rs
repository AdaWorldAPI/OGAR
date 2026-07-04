//! @generated DO-arm — OSM controller actions as `osm::<part_of>::<is_a>(input)`.
//! part_of = container module, is_a = action fn (standalone, not methods on the
//! data struct, per OGAR's ActionDef rule). Call: `osm::map::render(Input::default())`.

#![allow(clippy::all, dead_code, unused_variables)]

/// DO-arm action input — the Rails `params` / request. Typed-field harvest is a
/// follow-up (ruff `Function` carries reads/writes, not param types yet).
#[derive(Debug, Default)]
pub struct Input;

/// DO-arm action output — the Rails response.
#[derive(Debug, Default)]
pub struct Output;

pub mod account {
    use super::{Input, Output};
    /// `account:delete` — DO arm. Source: `AccountsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port AccountsController#destroy")
    }
    /// `account:show` — DO arm. Source: `AccountsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port AccountsController#show")
    }
    /// `account:update` — DO arm. Source: `AccountsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port AccountsController#update")
    }
}

pub mod active_list {
    use super::{Input, Output};
    /// `active_list:show` — DO arm. Source: `Api::UserBlocks::ActiveListsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserBlocks::ActiveListsController#show")
    }
}

pub mod advanced_preference {
    use super::{Input, Output};
    /// `advanced_preference:update_preferences` — DO arm. Source: `Preferences::AdvancedPreferencesController#update_preferences`.
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

pub mod basic_preference {
    use super::{Input, Output};
    /// `basic_preference:update_preferences` — DO arm. Source: `Preferences::BasicPreferencesController#update_preferences`.
    pub fn update_preferences(input: Input) -> Output {
        let _ = input;
        todo!("port Preferences::BasicPreferencesController#update_preferences")
    }
}

pub mod capabilitie {
    use super::{Input, Output};
    /// `capabilitie:show` — DO arm. Source: `Api::CapabilitiesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::CapabilitiesController#show")
    }
}

pub mod changeset {
    use super::{Input, Output};
    /// `changeset:conditions_bbox` — DO arm. Source: `Api::ChangesetsController#conditions_bbox`.
    pub fn conditions_bbox(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_bbox")
    }
    /// `changeset:conditions_closed` — DO arm. Source: `Api::ChangesetsController#conditions_closed`.
    pub fn conditions_closed(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_closed")
    }
    /// `changeset:conditions_ids` — DO arm. Source: `Api::ChangesetsController#conditions_ids`.
    pub fn conditions_ids(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_ids")
    }
    /// `changeset:conditions_nonempty` — DO arm. Source: `ChangesetsController#conditions_nonempty`.
    pub fn conditions_nonempty(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#conditions_nonempty")
    }
    /// `changeset:conditions_open` — DO arm. Source: `Api::ChangesetsController#conditions_open`.
    pub fn conditions_open(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_open")
    }
    /// `changeset:conditions_time` — DO arm. Source: `Api::ChangesetsController#conditions_time`.
    pub fn conditions_time(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_time")
    }
    /// `changeset:conditions_user` — DO arm. Source: `Api::ChangesetsController#conditions_user`.
    pub fn conditions_user(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#conditions_user")
    }
    /// `changeset:create` — DO arm. Source: `Api::ChangesetsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#create")
    }
    /// `changeset:feed` — DO arm. Source: `ChangesetsController#feed`.
    pub fn feed(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#feed")
    }
    /// `changeset:list` — DO arm. Source: `Api::ChangesetsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#index")
    }
    /// `changeset:load_nodes` — DO arm. Source: `ChangesetsController#load_nodes`.
    pub fn load_nodes(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#load_nodes")
    }
    /// `changeset:load_relations` — DO arm. Source: `ChangesetsController#load_relations`.
    pub fn load_relations(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#load_relations")
    }
    /// `changeset:load_ways` — DO arm. Source: `ChangesetsController#load_ways`.
    pub fn load_ways(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#load_ways")
    }
    /// `changeset:show` — DO arm. Source: `Api::ChangesetsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#show")
    }
    /// `changeset:update` — DO arm. Source: `Api::ChangesetsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetsController#update")
    }
    /// `changeset:wrap_lon` — DO arm. Source: `ChangesetsController#wrap_lon`.
    pub fn wrap_lon(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetsController#wrap_lon")
    }
}

pub mod changeset_comment {
    use super::{Input, Output};
    /// `changeset_comment:create` — DO arm. Source: `Api::ChangesetCommentsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetCommentsController#create")
    }
    /// `changeset_comment:list` — DO arm. Source: `Api::ChangesetCommentsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetCommentsController#index")
    }
    /// `changeset_comment:rate_limit_exceeded?` — DO arm. Source: `Api::ChangesetCommentsController#rate_limit_exceeded?`.
    pub fn rate_limit_exceeded(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetCommentsController#rate_limit_exceeded?")
    }
}

pub mod changeset_subscription {
    use super::{Input, Output};
    /// `changeset_subscription:create` — DO arm. Source: `Api::ChangesetSubscriptionsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetSubscriptionsController#create")
    }
    /// `changeset_subscription:delete` — DO arm. Source: `Api::ChangesetSubscriptionsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetSubscriptionsController#destroy")
    }
    /// `changeset_subscription:show` — DO arm. Source: `ChangesetSubscriptionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetSubscriptionsController#show")
    }
}

pub mod close {
    use super::{Input, Output};
    /// `close:update` — DO arm. Source: `Api::Changesets::ClosesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Changesets::ClosesController#update")
    }
}

pub mod companie {
    use super::{Input, Output};
    /// `companie:update_profile` — DO arm. Source: `Profiles::CompaniesController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::CompaniesController#update_profile")
    }
}

pub mod confirmation {
    use super::{Input, Output};
    /// `confirmation:confirm` — DO arm. Source: `ConfirmationsController#confirm`.
    pub fn confirm(input: Input) -> Output {
        let _ = input;
        todo!("port ConfirmationsController#confirm")
    }
    /// `confirmation:confirm_email` — DO arm. Source: `ConfirmationsController#confirm_email`.
    pub fn confirm_email(input: Input) -> Output {
        let _ = input;
        todo!("port ConfirmationsController#confirm_email")
    }
    /// `confirmation:confirm_resend` — DO arm. Source: `ConfirmationsController#confirm_resend`.
    pub fn confirm_resend(input: Input) -> Output {
        let _ = input;
        todo!("port ConfirmationsController#confirm_resend")
    }
    /// `confirmation:gravatar_status_message` — DO arm. Source: `ConfirmationsController#gravatar_status_message`.
    pub fn gravatar_status_message(input: Input) -> Output {
        let _ = input;
        todo!("port ConfirmationsController#gravatar_status_message")
    }
}

pub mod dashboard {
    use super::{Input, Output};
    /// `dashboard:show` — DO arm. Source: `DashboardsController#show`.
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
    /// `data:show` — DO arm. Source: `Api::Traces::DataController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Traces::DataController#show")
    }
}

pub mod deletion {
    use super::{Input, Output};
    /// `deletion:show` — DO arm. Source: `Accounts::DeletionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::DeletionsController#show")
    }
}

pub mod description {
    use super::{Input, Output};
    /// `description:update_profile` — DO arm. Source: `Profiles::DescriptionsController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::DescriptionsController#update_profile")
    }
}

pub mod diary_comment {
    use super::{Input, Output};
    /// `diary_comment:comment_params` — DO arm. Source: `DiaryCommentsController#comment_params`.
    pub fn comment_params(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryCommentsController#comment_params")
    }
    /// `diary_comment:create` — DO arm. Source: `DiaryCommentsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryCommentsController#create")
    }
    /// `diary_comment:hide` — DO arm. Source: `DiaryCommentsController#hide`.
    pub fn hide(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryCommentsController#hide")
    }
    /// `diary_comment:list` — DO arm. Source: `Users::DiaryCommentsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Users::DiaryCommentsController#index")
    }
    /// `diary_comment:unhide` — DO arm. Source: `DiaryCommentsController#unhide`.
    pub fn unhide(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryCommentsController#unhide")
    }
}

pub mod diary_entrie {
    use super::{Input, Output};
    /// `diary_entrie:create` — DO arm. Source: `DiaryEntriesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#create")
    }
    /// `diary_entrie:edit` — DO arm. Source: `DiaryEntriesController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#edit")
    }
    /// `diary_entrie:entry_params` — DO arm. Source: `DiaryEntriesController#entry_params`.
    pub fn entry_params(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#entry_params")
    }
    /// `diary_entrie:hide` — DO arm. Source: `DiaryEntriesController#hide`.
    pub fn hide(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#hide")
    }
    /// `diary_entrie:list` — DO arm. Source: `DiaryEntriesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#index")
    }
    /// `diary_entrie:new_form` — DO arm. Source: `DiaryEntriesController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#new")
    }
    /// `diary_entrie:rss` — DO arm. Source: `DiaryEntriesController#rss`.
    pub fn rss(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#rss")
    }
    /// `diary_entrie:set_map_location` — DO arm. Source: `DiaryEntriesController#set_map_location`.
    pub fn set_map_location(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#set_map_location")
    }
    /// `diary_entrie:show` — DO arm. Source: `DiaryEntriesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#show")
    }
    /// `diary_entrie:subscribe` — DO arm. Source: `DiaryEntriesController#subscribe`.
    pub fn subscribe(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#subscribe")
    }
    /// `diary_entrie:unhide` — DO arm. Source: `DiaryEntriesController#unhide`.
    pub fn unhide(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#unhide")
    }
    /// `diary_entrie:unsubscribe` — DO arm. Source: `DiaryEntriesController#unsubscribe`.
    pub fn unsubscribe(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#unsubscribe")
    }
    /// `diary_entrie:update` — DO arm. Source: `DiaryEntriesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port DiaryEntriesController#update")
    }
}

pub mod direction {
    use super::{Input, Output};
    /// `direction:show` — DO arm. Source: `DirectionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port DirectionsController#show")
    }
}

pub mod download {
    use super::{Input, Output};
    /// `download:show` — DO arm. Source: `Api::Changesets::DownloadsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Changesets::DownloadsController#show")
    }
    /// `download:show_redactions?` — DO arm. Source: `Api::Changesets::DownloadsController#show_redactions?`.
    pub fn show_redactions(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Changesets::DownloadsController#show_redactions?")
    }
}

pub mod error {
    use super::{Input, Output};
    /// `error:bad_request` — DO arm. Source: `ErrorsController#bad_request`.
    pub fn bad_request(input: Input) -> Output {
        let _ = input;
        todo!("port ErrorsController#bad_request")
    }
    /// `error:forbidden` — DO arm. Source: `ErrorsController#forbidden`.
    pub fn forbidden(input: Input) -> Output {
        let _ = input;
        todo!("port ErrorsController#forbidden")
    }
    /// `error:internal_server_error` — DO arm. Source: `ErrorsController#internal_server_error`.
    pub fn internal_server_error(input: Input) -> Output {
        let _ = input;
        todo!("port ErrorsController#internal_server_error")
    }
    /// `error:not_found` — DO arm. Source: `ErrorsController#not_found`.
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

pub mod feature_querie {
    use super::{Input, Output};
    /// `feature_querie:show` — DO arm. Source: `FeatureQueriesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port FeatureQueriesController#show")
    }
}

pub mod feed {
    use super::{Input, Output};
    /// `feed:show` — DO arm. Source: `ChangesetComments::FeedsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port ChangesetComments::FeedsController#show")
    }
}

pub mod follow {
    use super::{Input, Output};
    /// `follow:create` — DO arm. Source: `FollowsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port FollowsController#create")
    }
    /// `follow:delete` — DO arm. Source: `FollowsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port FollowsController#destroy")
    }
    /// `follow:show` — DO arm. Source: `FollowsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port FollowsController#show")
    }
}

pub mod heatmap {
    use super::{Input, Output};
    /// `heatmap:show` — DO arm. Source: `Users::HeatmapsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Users::HeatmapsController#show")
    }
    /// `heatmap:update_profile` — DO arm. Source: `Profiles::HeatmapsController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::HeatmapsController#update_profile")
    }
}

pub mod home {
    use super::{Input, Output};
    /// `home:show` — DO arm. Source: `Accounts::HomesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::HomesController#show")
    }
}

pub mod icon {
    use super::{Input, Output};
    /// `icon:show` — DO arm. Source: `Traces::IconsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Traces::IconsController#show")
    }
}

pub mod image {
    use super::{Input, Output};
    /// `image:update_profile` — DO arm. Source: `Profiles::ImagesController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::ImagesController#update_profile")
    }
}

pub mod inboxe {
    use super::{Input, Output};
    /// `inboxe:show` — DO arm. Source: `Api::Messages::InboxesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Messages::InboxesController#show")
    }
}

pub mod issue {
    use super::{Input, Output};
    /// `issue:find_issue` — DO arm. Source: `IssuesController#find_issue`.
    pub fn find_issue(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#find_issue")
    }
    /// `issue:ignore` — DO arm. Source: `IssuesController#ignore`.
    pub fn ignore(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#ignore")
    }
    /// `issue:list` — DO arm. Source: `IssuesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#index")
    }
    /// `issue:reopen` — DO arm. Source: `IssuesController#reopen`.
    pub fn reopen(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#reopen")
    }
    /// `issue:resolve` — DO arm. Source: `IssuesController#resolve`.
    pub fn resolve(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#resolve")
    }
    /// `issue:show` — DO arm. Source: `IssuesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port IssuesController#show")
    }
}

pub mod issue_comment {
    use super::{Input, Output};
    /// `issue_comment:create` — DO arm. Source: `IssueCommentsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port IssueCommentsController#create")
    }
    /// `issue_comment:issue_comment_params` — DO arm. Source: `IssueCommentsController#issue_comment_params`.
    pub fn issue_comment_params(input: Input) -> Output {
        let _ = input;
        todo!("port IssueCommentsController#issue_comment_params")
    }
    /// `issue_comment:reassign_issue` — DO arm. Source: `IssueCommentsController#reassign_issue`.
    pub fn reassign_issue(input: Input) -> Output {
        let _ = input;
        todo!("port IssueCommentsController#reassign_issue")
    }
}

pub mod issued_block {
    use super::{Input, Output};
    /// `issued_block:show` — DO arm. Source: `Users::IssuedBlocksController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Users::IssuedBlocksController#show")
    }
}

pub mod languages_pane {
    use super::{Input, Output};
    /// `languages_pane:show` — DO arm. Source: `LanguagesPanesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port LanguagesPanesController#show")
    }
}

pub mod latlon_querie {
    use super::{Input, Output};
    /// `latlon_querie:create` — DO arm. Source: `Searches::LatlonQueriesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Searches::LatlonQueriesController#create")
    }
}

pub mod layers_pane {
    use super::{Input, Output};
    /// `layers_pane:show` — DO arm. Source: `LayersPanesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port LayersPanesController#show")
    }
}

pub mod legend_pane {
    use super::{Input, Output};
    /// `legend_pane:show` — DO arm. Source: `LegendPanesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port LegendPanesController#show")
    }
}

pub mod link {
    use super::{Input, Output};
    /// `link:update_profile` — DO arm. Source: `Profiles::LinksController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::LinksController#update_profile")
    }
}

pub mod list {
    use super::{Input, Output};
    /// `list:show` — DO arm. Source: `Users::ListsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Users::ListsController#show")
    }
    /// `list:update` — DO arm. Source: `Users::ListsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Users::ListsController#update")
    }
}

pub mod location {
    use super::{Input, Output};
    /// `location:show` — DO arm. Source: `Profiles::LocationsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::LocationsController#show")
    }
    /// `location:update_profile` — DO arm. Source: `Profiles::LocationsController#update_profile`.
    pub fn update_profile(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::LocationsController#update_profile")
    }
}

pub mod mailboxe {
    use super::{Input, Output};
    /// `mailboxe:show_messages` — DO arm. Source: `Api::Messages::MailboxesController#show_messages`.
    pub fn show_messages(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Messages::MailboxesController#show_messages")
    }
}

pub mod map {
    use super::{Input, Output};
    /// `map:show` — DO arm. Source: `Api::MapsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::MapsController#show")
    }
}

pub mod message {
    use super::{Input, Output};
    /// `message:create` — DO arm. Source: `Api::MessagesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::MessagesController#create")
    }
    /// `message:delete` — DO arm. Source: `Api::MessagesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::MessagesController#destroy")
    }
    /// `message:message_params` — DO arm. Source: `MessagesController#message_params`.
    pub fn message_params(input: Input) -> Output {
        let _ = input;
        todo!("port MessagesController#message_params")
    }
    /// `message:new_form` — DO arm. Source: `MessagesController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port MessagesController#new")
    }
    /// `message:show` — DO arm. Source: `Api::MessagesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::MessagesController#show")
    }
    /// `message:update` — DO arm. Source: `Api::MessagesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::MessagesController#update")
    }
}

pub mod mute {
    use super::{Input, Output};
    /// `mute:delete` — DO arm. Source: `Messages::MutesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::MutesController#destroy")
    }
}

pub mod muted_inboxe {
    use super::{Input, Output};
    /// `muted_inboxe:show` — DO arm. Source: `Messages::MutedInboxesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::MutedInboxesController#show")
    }
}

pub mod node {
    use super::{Input, Output};
    /// `node:create` — DO arm. Source: `Api::NodesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NodesController#create")
    }
    /// `node:delete` — DO arm. Source: `Api::NodesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NodesController#destroy")
    }
    /// `node:list` — DO arm. Source: `Api::NodesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NodesController#index")
    }
    /// `node:show` — DO arm. Source: `Api::NodesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NodesController#show")
    }
    /// `node:update` — DO arm. Source: `Api::NodesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NodesController#update")
    }
}

pub mod nominatim_querie {
    use super::{Input, Output};
    /// `nominatim_querie:create` — DO arm. Source: `Searches::NominatimQueriesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Searches::NominatimQueriesController#create")
    }
}

pub mod nominatim_reverse_querie {
    use super::{Input, Output};
    /// `nominatim_reverse_querie:create` — DO arm. Source: `Searches::NominatimReverseQueriesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Searches::NominatimReverseQueriesController#create")
    }
}

pub mod note {
    use super::{Input, Output};
    /// `note:add_comment` — DO arm. Source: `Api::NotesController#add_comment`.
    pub fn add_comment(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#add_comment")
    }
    /// `note:author_info` — DO arm. Source: `Api::NotesController#author_info`.
    pub fn author_info(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#author_info")
    }
    /// `note:bbox_condition` — DO arm. Source: `Api::NotesController#bbox_condition`.
    pub fn bbox_condition(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#bbox_condition")
    }
    /// `note:close` — DO arm. Source: `Api::NotesController#close`.
    pub fn close(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#close")
    }
    /// `note:closed_condition` — DO arm. Source: `Api::NotesController#closed_condition`.
    pub fn closed_condition(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#closed_condition")
    }
    /// `note:comment` — DO arm. Source: `Api::NotesController#comment`.
    pub fn comment(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#comment")
    }
    /// `note:create` — DO arm. Source: `Api::NotesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#create")
    }
    /// `note:delete` — DO arm. Source: `Api::NotesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#destroy")
    }
    /// `note:feed` — DO arm. Source: `Api::NotesController#feed`.
    pub fn feed(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#feed")
    }
    /// `note:list` — DO arm. Source: `Api::NotesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#index")
    }
    /// `note:new_form` — DO arm. Source: `NotesController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port NotesController#new")
    }
    /// `note:reopen` — DO arm. Source: `Api::NotesController#reopen`.
    pub fn reopen(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#reopen")
    }
    /// `note:search` — DO arm. Source: `Api::NotesController#search`.
    pub fn search(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#search")
    }
    /// `note:show` — DO arm. Source: `Api::NotesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NotesController#show")
    }
}

pub mod note_subscription {
    use super::{Input, Output};
    /// `note_subscription:create` — DO arm. Source: `Api::NoteSubscriptionsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NoteSubscriptionsController#create")
    }
    /// `note_subscription:delete` — DO arm. Source: `Api::NoteSubscriptionsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::NoteSubscriptionsController#destroy")
    }
}

pub mod notification_preference {
    use super::{Input, Output};
    /// `notification_preference:update_preferences` — DO arm. Source: `Preferences::NotificationPreferencesController#update_preferences`.
    pub fn update_preferences(input: Input) -> Output {
        let _ = input;
        todo!("port Preferences::NotificationPreferencesController#update_preferences")
    }
}

pub mod oauth2_application {
    use super::{Input, Output};
    /// `oauth2_application:application_params` — DO arm. Source: `Oauth2ApplicationsController#application_params`.
    pub fn application_params(input: Input) -> Output {
        let _ = input;
        todo!("port Oauth2ApplicationsController#application_params")
    }
    /// `oauth2_application:list` — DO arm. Source: `Oauth2ApplicationsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Oauth2ApplicationsController#index")
    }
    /// `oauth2_application:set_application` — DO arm. Source: `Oauth2ApplicationsController#set_application`.
    pub fn set_application(input: Input) -> Output {
        let _ = input;
        todo!("port Oauth2ApplicationsController#set_application")
    }
}

pub mod old_element {
    use super::{Input, Output};
    /// `old_element:list` — DO arm. Source: `Api::OldElementsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldElementsController#index")
    }
    /// `old_element:require_moderator_for_unredacted_history` — DO arm. Source: `OldElementsController#require_moderator_for_unredacted_history`.
    pub fn require_moderator_for_unredacted_history(input: Input) -> Output {
        let _ = input;
        todo!("port OldElementsController#require_moderator_for_unredacted_history")
    }
    /// `old_element:show` — DO arm. Source: `Api::OldElementsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldElementsController#show")
    }
    /// `old_element:show_redactions?` — DO arm. Source: `Api::OldElementsController#show_redactions?`.
    pub fn show_redactions(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldElementsController#show_redactions?")
    }
}

pub mod old_node {
    use super::{Input, Output};
    /// `old_node:list` — DO arm. Source: `OldNodesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port OldNodesController#index")
    }
    /// `old_node:lookup_old_element` — DO arm. Source: `Api::OldNodesController#lookup_old_element`.
    pub fn lookup_old_element(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldNodesController#lookup_old_element")
    }
    /// `old_node:lookup_old_element_versions` — DO arm. Source: `Api::OldNodesController#lookup_old_element_versions`.
    pub fn lookup_old_element_versions(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldNodesController#lookup_old_element_versions")
    }
    /// `old_node:show` — DO arm. Source: `OldNodesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port OldNodesController#show")
    }
}

pub mod old_relation {
    use super::{Input, Output};
    /// `old_relation:list` — DO arm. Source: `OldRelationsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port OldRelationsController#index")
    }
    /// `old_relation:lookup_old_element` — DO arm. Source: `Api::OldRelationsController#lookup_old_element`.
    pub fn lookup_old_element(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldRelationsController#lookup_old_element")
    }
    /// `old_relation:lookup_old_element_versions` — DO arm. Source: `Api::OldRelationsController#lookup_old_element_versions`.
    pub fn lookup_old_element_versions(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldRelationsController#lookup_old_element_versions")
    }
    /// `old_relation:show` — DO arm. Source: `OldRelationsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port OldRelationsController#show")
    }
}

pub mod old_relation_member {
    use super::{Input, Output};
    /// `old_relation_member:show` — DO arm. Source: `OldRelationMembersController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port OldRelationMembersController#show")
    }
}

pub mod old_way {
    use super::{Input, Output};
    /// `old_way:list` — DO arm. Source: `OldWaysController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port OldWaysController#index")
    }
    /// `old_way:lookup_old_element` — DO arm. Source: `Api::OldWaysController#lookup_old_element`.
    pub fn lookup_old_element(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldWaysController#lookup_old_element")
    }
    /// `old_way:lookup_old_element_versions` — DO arm. Source: `Api::OldWaysController#lookup_old_element_versions`.
    pub fn lookup_old_element_versions(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldWaysController#lookup_old_element_versions")
    }
    /// `old_way:show` — DO arm. Source: `OldWaysController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port OldWaysController#show")
    }
}

pub mod outboxe {
    use super::{Input, Output};
    /// `outboxe:show` — DO arm. Source: `Api::Messages::OutboxesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Messages::OutboxesController#show")
    }
}

pub mod password {
    use super::{Input, Output};
    /// `password:create` — DO arm. Source: `PasswordsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port PasswordsController#create")
    }
    /// `password:edit` — DO arm. Source: `PasswordsController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port PasswordsController#edit")
    }
    /// `password:new_form` — DO arm. Source: `PasswordsController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port PasswordsController#new")
    }
    /// `password:update` — DO arm. Source: `PasswordsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port PasswordsController#update")
    }
}

pub mod pd_declaration {
    use super::{Input, Output};
    /// `pd_declaration:create` — DO arm. Source: `Accounts::PdDeclarationsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::PdDeclarationsController#create")
    }
    /// `pd_declaration:show` — DO arm. Source: `Accounts::PdDeclarationsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::PdDeclarationsController#show")
    }
}

pub mod permission {
    use super::{Input, Output};
    /// `permission:show` — DO arm. Source: `Api::PermissionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::PermissionsController#show")
    }
}

pub mod picture {
    use super::{Input, Output};
    /// `picture:show` — DO arm. Source: `Traces::PicturesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Traces::PicturesController#show")
    }
}

pub mod preference {
    use super::{Input, Output};
    /// `preference:show` — DO arm. Source: `Preferences::PreferencesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Preferences::PreferencesController#show")
    }
    /// `preference:update` — DO arm. Source: `Preferences::PreferencesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Preferences::PreferencesController#update")
    }
}

pub mod profile_section {
    use super::{Input, Output};
    /// `profile_section:show` — DO arm. Source: `Profiles::ProfileSectionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::ProfileSectionsController#show")
    }
    /// `profile_section:update` — DO arm. Source: `Profiles::ProfileSectionsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Profiles::ProfileSectionsController#update")
    }
}

pub mod querie {
    use super::{Input, Output};
    /// `querie:fetch_text` — DO arm. Source: `Searches::QueriesController#fetch_text`.
    pub fn fetch_text(input: Input) -> Output {
        let _ = input;
        todo!("port Searches::QueriesController#fetch_text")
    }
    /// `querie:fetch_xml` — DO arm. Source: `Searches::QueriesController#fetch_xml`.
    pub fn fetch_xml(input: Input) -> Output {
        let _ = input;
        todo!("port Searches::QueriesController#fetch_xml")
    }
}

pub mod read_mark {
    use super::{Input, Output};
    /// `read_mark:create` — DO arm. Source: `Messages::ReadMarksController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::ReadMarksController#create")
    }
    /// `read_mark:delete` — DO arm. Source: `Messages::ReadMarksController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::ReadMarksController#destroy")
    }
    /// `read_mark:mark` — DO arm. Source: `Messages::ReadMarksController#mark`.
    pub fn mark(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::ReadMarksController#mark")
    }
}

pub mod received_block {
    use super::{Input, Output};
    /// `received_block:delete` — DO arm. Source: `Users::ReceivedBlocksController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Users::ReceivedBlocksController#destroy")
    }
    /// `received_block:edit` — DO arm. Source: `Users::ReceivedBlocksController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port Users::ReceivedBlocksController#edit")
    }
    /// `received_block:show` — DO arm. Source: `Users::ReceivedBlocksController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Users::ReceivedBlocksController#show")
    }
}

pub mod redaction {
    use super::{Input, Output};
    /// `redaction:create` — DO arm. Source: `Api::OldElements::RedactionsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldElements::RedactionsController#create")
    }
    /// `redaction:delete` — DO arm. Source: `Api::OldElements::RedactionsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldElements::RedactionsController#destroy")
    }
    /// `redaction:edit` — DO arm. Source: `RedactionsController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#edit")
    }
    /// `redaction:list` — DO arm. Source: `RedactionsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#index")
    }
    /// `redaction:lookup_old_element` — DO arm. Source: `Api::OldNodes::RedactionsController#lookup_old_element`.
    pub fn lookup_old_element(input: Input) -> Output {
        let _ = input;
        todo!("port Api::OldNodes::RedactionsController#lookup_old_element")
    }
    /// `redaction:lookup_redaction` — DO arm. Source: `RedactionsController#lookup_redaction`.
    pub fn lookup_redaction(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#lookup_redaction")
    }
    /// `redaction:new_form` — DO arm. Source: `RedactionsController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#new")
    }
    /// `redaction:show` — DO arm. Source: `RedactionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#show")
    }
    /// `redaction:update` — DO arm. Source: `RedactionsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port RedactionsController#update")
    }
}

pub mod relation {
    use super::{Input, Output};
    /// `relation:create` — DO arm. Source: `Api::RelationsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::RelationsController#create")
    }
    /// `relation:delete` — DO arm. Source: `Api::RelationsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::RelationsController#destroy")
    }
    /// `relation:list` — DO arm. Source: `Api::Nodes::RelationsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Nodes::RelationsController#index")
    }
    /// `relation:show` — DO arm. Source: `Api::RelationsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::RelationsController#show")
    }
    /// `relation:update` — DO arm. Source: `Api::RelationsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::RelationsController#update")
    }
}

pub mod relation_member {
    use super::{Input, Output};
    /// `relation_member:show` — DO arm. Source: `RelationMembersController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port RelationMembersController#show")
    }
}

pub mod replie {
    use super::{Input, Output};
    /// `replie:new_form` — DO arm. Source: `Messages::RepliesController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port Messages::RepliesController#new")
    }
}

pub mod report {
    use super::{Input, Output};
    /// `report:create` — DO arm. Source: `ReportsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#create")
    }
    /// `report:create_new_report_params` — DO arm. Source: `ReportsController#create_new_report_params`.
    pub fn create_new_report_params(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#create_new_report_params")
    }
    /// `report:default_assigned_role` — DO arm. Source: `ReportsController#default_assigned_role`.
    pub fn default_assigned_role(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#default_assigned_role")
    }
    /// `report:issue_params` — DO arm. Source: `ReportsController#issue_params`.
    pub fn issue_params(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#issue_params")
    }
    /// `report:new_form` — DO arm. Source: `ReportsController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#new")
    }
    /// `report:report_params` — DO arm. Source: `ReportsController#report_params`.
    pub fn report_params(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#report_params")
    }
    /// `report:required_new_report_params_present?` — DO arm. Source: `ReportsController#required_new_report_params_present?`.
    pub fn required_new_report_params_present(input: Input) -> Output {
        let _ = input;
        todo!("port ReportsController#required_new_report_params_present?")
    }
}

pub mod reporter {
    use super::{Input, Output};
    /// `reporter:list` — DO arm. Source: `Issues::ReportersController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Issues::ReportersController#index")
    }
}

pub mod searche {
    use super::{Input, Output};
    /// `searche:dms_regexp` — DO arm. Source: `SearchesController#dms_regexp`.
    pub fn dms_regexp(input: Input) -> Output {
        let _ = input;
        todo!("port SearchesController#dms_regexp")
    }
    /// `searche:normalize_params` — DO arm. Source: `SearchesController#normalize_params`.
    pub fn normalize_params(input: Input) -> Output {
        let _ = input;
        todo!("port SearchesController#normalize_params")
    }
    /// `searche:show` — DO arm. Source: `SearchesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port SearchesController#show")
    }
    /// `searche:to_decdeg` — DO arm. Source: `SearchesController#to_decdeg`.
    pub fn to_decdeg(input: Input) -> Output {
        let _ = input;
        todo!("port SearchesController#to_decdeg")
    }
}

pub mod session {
    use super::{Input, Output};
    /// `session:create` — DO arm. Source: `SessionsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port SessionsController#create")
    }
    /// `session:delete` — DO arm. Source: `SessionsController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port SessionsController#destroy")
    }
    /// `session:new_form` — DO arm. Source: `SessionsController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port SessionsController#new")
    }
    /// `session:password_authentication` — DO arm. Source: `SessionsController#password_authentication`.
    pub fn password_authentication(input: Input) -> Output {
        let _ = input;
        todo!("port SessionsController#password_authentication")
    }
}

pub mod share_pane {
    use super::{Input, Output};
    /// `share_pane:show` — DO arm. Source: `SharePanesController#show`.
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

pub mod statuse {
    use super::{Input, Output};
    /// `statuse:lookup_user_by_name` — DO arm. Source: `Users::StatusesController#lookup_user_by_name`.
    pub fn lookup_user_by_name(input: Input) -> Output {
        let _ = input;
        todo!("port Users::StatusesController#lookup_user_by_name")
    }
    /// `statuse:update` — DO arm. Source: `Users::StatusesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Users::StatusesController#update")
    }
}

pub mod term {
    use super::{Input, Output};
    /// `term:show` — DO arm. Source: `Accounts::TermsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::TermsController#show")
    }
    /// `term:update` — DO arm. Source: `Accounts::TermsController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Accounts::TermsController#update")
    }
}

pub mod trace {
    use super::{Input, Output};
    /// `trace:create` — DO arm. Source: `Api::TracesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#create")
    }
    /// `trace:default_visibility` — DO arm. Source: `TracesController#default_visibility`.
    pub fn default_visibility(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#default_visibility")
    }
    /// `trace:delete` — DO arm. Source: `Api::TracesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#destroy")
    }
    /// `trace:do_create` — DO arm. Source: `Api::TracesController#do_create`.
    pub fn do_create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#do_create")
    }
    /// `trace:edit` — DO arm. Source: `TracesController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#edit")
    }
    /// `trace:list` — DO arm. Source: `Api::Users::TracesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Users::TracesController#index")
    }
    /// `trace:mine` — DO arm. Source: `TracesController#mine`.
    pub fn mine(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#mine")
    }
    /// `trace:new_form` — DO arm. Source: `TracesController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#new")
    }
    /// `trace:offline_error` — DO arm. Source: `Api::TracesController#offline_error`.
    pub fn offline_error(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#offline_error")
    }
    /// `trace:offline_redirect` — DO arm. Source: `TracesController#offline_redirect`.
    pub fn offline_redirect(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#offline_redirect")
    }
    /// `trace:offline_warning` — DO arm. Source: `TracesController#offline_warning`.
    pub fn offline_warning(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#offline_warning")
    }
    /// `trace:show` — DO arm. Source: `Api::TracesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#show")
    }
    /// `trace:trace_params` — DO arm. Source: `TracesController#trace_params`.
    pub fn trace_params(input: Input) -> Output {
        let _ = input;
        todo!("port TracesController#trace_params")
    }
    /// `trace:update` — DO arm. Source: `Api::TracesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracesController#update")
    }
}

pub mod tracepoint {
    use super::{Input, Output};
    /// `tracepoint:list` — DO arm. Source: `Api::TracepointsController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::TracepointsController#index")
    }
}

pub mod upload {
    use super::{Input, Output};
    /// `upload:create` — DO arm. Source: `Api::Changesets::UploadsController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Changesets::UploadsController#create")
    }
}

pub mod user {
    use super::{Input, Output};
    /// `user:auth_failure` — DO arm. Source: `UsersController#auth_failure`.
    pub fn auth_failure(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#auth_failure")
    }
    /// `user:auth_success` — DO arm. Source: `UsersController#auth_success`.
    pub fn auth_success(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#auth_success")
    }
    /// `user:check_signup_allowed?` — DO arm. Source: `UsersController#check_signup_allowed?`.
    pub fn check_signup_allowed(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#check_signup_allowed?")
    }
    /// `user:create` — DO arm. Source: `UsersController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#create")
    }
    /// `user:details` — DO arm. Source: `Api::UsersController#details`.
    pub fn details(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UsersController#details")
    }
    /// `user:go_public` — DO arm. Source: `UsersController#go_public`.
    pub fn go_public(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#go_public")
    }
    /// `user:list` — DO arm. Source: `Api::UsersController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UsersController#index")
    }
    /// `user:new_form` — DO arm. Source: `UsersController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#new")
    }
    /// `user:save_new_user` — DO arm. Source: `UsersController#save_new_user`.
    pub fn save_new_user(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#save_new_user")
    }
    /// `user:show` — DO arm. Source: `Api::UsersController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UsersController#show")
    }
    /// `user:user_params` — DO arm. Source: `UsersController#user_params`.
    pub fn user_params(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#user_params")
    }
    /// `user:valid_turnstile_response?` — DO arm. Source: `UsersController#valid_turnstile_response?`.
    pub fn valid_turnstile_response(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#valid_turnstile_response?")
    }
    /// `user:welcome_options` — DO arm. Source: `UsersController#welcome_options`.
    pub fn welcome_options(input: Input) -> Output {
        let _ = input;
        todo!("port UsersController#welcome_options")
    }
}

pub mod user_block {
    use super::{Input, Output};
    /// `user_block:create` — DO arm. Source: `Api::UserBlocksController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserBlocksController#create")
    }
    /// `user_block:edit` — DO arm. Source: `UserBlocksController#edit`.
    pub fn edit(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#edit")
    }
    /// `user_block:list` — DO arm. Source: `UserBlocksController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#index")
    }
    /// `user_block:lookup_user_block` — DO arm. Source: `UserBlocksController#lookup_user_block`.
    pub fn lookup_user_block(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#lookup_user_block")
    }
    /// `user_block:new_form` — DO arm. Source: `UserBlocksController#new`.
    pub fn new_form(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#new")
    }
    /// `user_block:require_valid_params` — DO arm. Source: `UserBlocksController#require_valid_params`.
    pub fn require_valid_params(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#require_valid_params")
    }
    /// `user_block:show` — DO arm. Source: `Api::UserBlocksController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserBlocksController#show")
    }
    /// `user_block:update` — DO arm. Source: `UserBlocksController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port UserBlocksController#update")
    }
}

pub mod user_mute {
    use super::{Input, Output};
    /// `user_mute:create` — DO arm. Source: `UserMutesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port UserMutesController#create")
    }
    /// `user_mute:delete` — DO arm. Source: `UserMutesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port UserMutesController#destroy")
    }
    /// `user_mute:list` — DO arm. Source: `UserMutesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port UserMutesController#index")
    }
}

pub mod user_preference {
    use super::{Input, Output};
    /// `user_preference:delete` — DO arm. Source: `Api::UserPreferencesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserPreferencesController#destroy")
    }
    /// `user_preference:list` — DO arm. Source: `Api::UserPreferencesController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserPreferencesController#index")
    }
    /// `user_preference:show` — DO arm. Source: `Api::UserPreferencesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserPreferencesController#show")
    }
    /// `user_preference:update` — DO arm. Source: `Api::UserPreferencesController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserPreferencesController#update")
    }
    /// `user_preference:update_all` — DO arm. Source: `Api::UserPreferencesController#update_all`.
    pub fn update_all(input: Input) -> Output {
        let _ = input;
        todo!("port Api::UserPreferencesController#update_all")
    }
}

pub mod user_role {
    use super::{Input, Output};
    /// `user_role:create` — DO arm. Source: `UserRolesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port UserRolesController#create")
    }
    /// `user_role:delete` — DO arm. Source: `UserRolesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port UserRolesController#destroy")
    }
    /// `user_role:in_role` — DO arm. Source: `UserRolesController#in_role`.
    pub fn in_role(input: Input) -> Output {
        let _ = input;
        todo!("port UserRolesController#in_role")
    }
    /// `user_role:not_in_role` — DO arm. Source: `UserRolesController#not_in_role`.
    pub fn not_in_role(input: Input) -> Output {
        let _ = input;
        todo!("port UserRolesController#not_in_role")
    }
    /// `user_role:require_valid_role` — DO arm. Source: `UserRolesController#require_valid_role`.
    pub fn require_valid_role(input: Input) -> Output {
        let _ = input;
        todo!("port UserRolesController#require_valid_role")
    }
}

pub mod version {
    use super::{Input, Output};
    /// `version:show` — DO arm. Source: `Api::VersionsController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::VersionsController#show")
    }
}

pub mod visibilitie {
    use super::{Input, Output};
    /// `visibilitie:create` — DO arm. Source: `Api::ChangesetComments::VisibilitiesController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetComments::VisibilitiesController#create")
    }
    /// `visibilitie:delete` — DO arm. Source: `Api::ChangesetComments::VisibilitiesController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::ChangesetComments::VisibilitiesController#destroy")
    }
}

pub mod way {
    use super::{Input, Output};
    /// `way:create` — DO arm. Source: `Api::WaysController#create`.
    pub fn create(input: Input) -> Output {
        let _ = input;
        todo!("port Api::WaysController#create")
    }
    /// `way:delete` — DO arm. Source: `Api::WaysController#destroy`.
    pub fn delete(input: Input) -> Output {
        let _ = input;
        todo!("port Api::WaysController#destroy")
    }
    /// `way:list` — DO arm. Source: `Api::Nodes::WaysController#index`.
    pub fn list(input: Input) -> Output {
        let _ = input;
        todo!("port Api::Nodes::WaysController#index")
    }
    /// `way:show` — DO arm. Source: `Api::WaysController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port Api::WaysController#show")
    }
    /// `way:update` — DO arm. Source: `Api::WaysController#update`.
    pub fn update(input: Input) -> Output {
        let _ = input;
        todo!("port Api::WaysController#update")
    }
}

pub mod webgl_error_pane {
    use super::{Input, Output};
    /// `webgl_error_pane:show` — DO arm. Source: `WebglErrorPanesController#show`.
    pub fn show(input: Input) -> Output {
        let _ = input;
        todo!("port WebglErrorPanesController#show")
    }
}

