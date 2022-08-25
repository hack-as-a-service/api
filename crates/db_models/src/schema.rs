table! {
		apps (id) {
				id -> Int4,
				created_at -> Timestamp,
				slug -> Text,
				team_id -> Int4,
				enabled -> Bool,
				container_id -> Nullable<Text>,
				network_id -> Nullable<Text>,
		}
}

table! {
		builds (id) {
				id -> Int4,
				started_at -> Timestamp,
				ended_at -> Nullable<Timestamp>,
				events -> Array<Text>,
				app_id -> Int4,
		}
}

table! {
		domains (id) {
				id -> Int4,
				domain -> Text,
				verified -> Bool,
				app_id -> Int4,
		}
}

table! {
		oauth_apps (client_id) {
				client_id -> Text,
				name -> Text,
		}
}

table! {
		oauth_device_requests (id) {
				id -> Int4,
				created_at -> Timestamp,
				expires_at -> Timestamp,
				oauth_app_id -> Text,
				token -> Nullable<Text>,
				device_code -> Text,
				user_code -> Text,
				token_retrieved -> Bool,
				access_denied -> Bool,
		}
}

table! {
		team_users (team_id, user_id) {
				user_id -> Int4,
				team_id -> Int4,
		}
}

table! {
		teams (id) {
				id -> Int4,
				created_at -> Timestamp,
				name -> Nullable<Text>,
				avatar -> Nullable<Text>,
				personal -> Bool,
				slug -> Text,
				invite -> Text,
		}
}

table! {
		tokens (token) {
				token -> Text,
				created_at -> Timestamp,
				expires_at -> Timestamp,
				user_id -> Int4,
		}
}

table! {
		users (id) {
				id -> Int4,
				created_at -> Timestamp,
				slack_user_id -> Text,
				name -> Text,
				avatar -> Nullable<Text>,
		}
}

table! {
		whitelist (slack_user_id) {
				slack_user_id -> Text,
		}
}

joinable!(apps -> teams (team_id));
joinable!(builds -> apps (app_id));
joinable!(domains -> apps (app_id));
joinable!(oauth_device_requests -> oauth_apps (oauth_app_id));
joinable!(oauth_device_requests -> tokens (token));
joinable!(team_users -> teams (team_id));
joinable!(team_users -> users (user_id));
joinable!(tokens -> users (user_id));

allow_tables_to_appear_in_same_query!(
	apps,
	builds,
	domains,
	oauth_apps,
	oauth_device_requests,
	team_users,
	teams,
	tokens,
	users,
	whitelist,
);
