//! Objetos TL construidos a mano (capa 230 de grammers-tl-types 0.11) para las pruebas.
use grammers_tl_types as tl;

pub fn user(id: i64, first: &str, hash: i64, is_self: bool) -> tl::types::User {
    tl::types::User {
        is_self,
        contact: !is_self,
        mutual_contact: false,
        deleted: false,
        bot: false,
        bot_chat_history: false,
        bot_nochats: false,
        verified: false,
        restricted: false,
        min: false,
        bot_inline_geo: false,
        support: false,
        scam: false,
        apply_min_photo: false,
        fake: false,
        bot_attach_menu: false,
        premium: false,
        attach_menu_enabled: false,
        bot_can_edit: false,
        close_friend: false,
        stories_hidden: false,
        stories_unavailable: false,
        contact_require_premium: false,
        bot_business: false,
        bot_has_main_app: false,
        bot_forum_view: false,
        bot_forum_can_manage_topics: false,
        bot_can_manage_bots: false,
        bot_guestchat: false,
        bot_guard: false,
        id,
        access_hash: Some(hash),
        first_name: Some(first.into()),
        last_name: None,
        username: None,
        phone: None,
        photo: None,
        status: Some(tl::types::UserStatusOffline { was_online: 1_700_000_000 }.into()),
        bot_info_version: None,
        restriction_reason: None,
        bot_inline_placeholder: None,
        lang_code: None,
        emoji_status: None,
        usernames: None,
        stories_max_id: None,
        color: None,
        profile_color: None,
        bot_active_users: None,
        bot_verification_icon: None,
        send_paid_messages_stars: None,
        linked_community_id: None,
    }
}

pub fn user_enum(id: i64, first: &str, hash: i64, is_self: bool) -> tl::enums::User {
    user(id, first, hash, is_self).into()
}

pub fn channel(id: i64, title: &str, broadcast: bool, hash: i64) -> tl::enums::Chat {
    tl::types::Channel {
        creator: false,
        left: false,
        broadcast,
        verified: false,
        megagroup: !broadcast,
        restricted: false,
        signatures: false,
        min: false,
        scam: false,
        has_link: false,
        has_geo: false,
        slowmode_enabled: false,
        call_active: false,
        call_not_empty: false,
        fake: false,
        gigagroup: false,
        noforwards: false,
        join_to_send: false,
        join_request: false,
        forum: false,
        stories_hidden: false,
        stories_hidden_min: false,
        stories_unavailable: false,
        signature_profiles: false,
        autotranslation: false,
        broadcast_messages_allowed: false,
        monoforum: false,
        forum_tabs: false,
        id,
        access_hash: Some(hash),
        title: title.into(),
        username: Some("canal_prueba".into()),
        photo: tl::enums::ChatPhoto::Empty,
        date: 0,
        restriction_reason: None,
        admin_rights: None,
        banned_rights: None,
        default_banned_rights: None,
        participants_count: Some(321),
        usernames: None,
        stories_max_id: None,
        color: None,
        profile_color: None,
        emoji_status: None,
        level: None,
        subscription_until_date: None,
        bot_verification_icon: None,
        send_paid_messages_stars: None,
        linked_monoforum_id: None,
        linked_community_id: None,
    }
    .into()
}

pub fn chat(id: i64, title: &str) -> tl::enums::Chat {
    tl::types::Chat {
        creator: false,
        left: false,
        deactivated: false,
        call_active: false,
        call_not_empty: false,
        noforwards: false,
        id,
        title: title.into(),
        photo: tl::enums::ChatPhoto::Empty,
        participants_count: 4,
        date: 0,
        version: 1,
        migrated_to: None,
        admin_rights: None,
        default_banned_rights: None,
    }
    .into()
}

pub fn peer_user(id: i64) -> tl::enums::Peer {
    tl::types::PeerUser { user_id: id }.into()
}
pub fn peer_chat(id: i64) -> tl::enums::Peer {
    tl::types::PeerChat { chat_id: id }.into()
}
pub fn peer_channel(id: i64) -> tl::enums::Peer {
    tl::types::PeerChannel { channel_id: id }.into()
}

pub fn message(id: i32, peer: tl::enums::Peer, text: &str, out: bool, from: Option<tl::enums::Peer>, date: i32) -> tl::types::Message {
    tl::types::Message {
        out,
        mentioned: false,
        media_unread: false,
        silent: false,
        post: false,
        from_scheduled: false,
        legacy: false,
        edit_hide: false,
        pinned: false,
        noforwards: false,
        invert_media: false,
        offline: false,
        video_processing_pending: false,
        paid_suggested_post_stars: false,
        paid_suggested_post_ton: false,
        id,
        from_id: from,
        from_boosts_applied: None,
        from_rank: None,
        peer_id: peer,
        saved_peer_id: None,
        fwd_from: None,
        via_bot_id: None,
        via_business_bot_id: None,
        guestchat_via_from: None,
        reply_to: None,
        date,
        message: text.into(),
        media: None,
        reply_markup: None,
        entities: None,
        views: None,
        forwards: None,
        replies: None,
        edit_date: None,
        post_author: None,
        grouped_id: None,
        reactions: None,
        restriction_reason: None,
        ttl_period: None,
        quick_reply_shortcut_id: None,
        effect: None,
        factcheck: None,
        report_delivery_until_date: None,
        paid_message_stars: None,
        suggested_post: None,
        schedule_repeat_period: None,
        summary_from_language: None,
        rich_message: None,
    }
}

pub fn msg_enum(id: i32, peer: tl::enums::Peer, text: &str, out: bool, date: i32) -> tl::enums::Message {
    message(id, peer, text, out, None, date).into()
}

pub fn notify(mute_until: Option<i32>) -> tl::enums::PeerNotifySettings {
    tl::types::PeerNotifySettings {
        show_previews: None,
        silent: None,
        mute_until,
        ios_sound: None,
        android_sound: None,
        other_sound: None,
        stories_muted: None,
        stories_hide_sender: None,
        stories_ios_sound: None,
        stories_android_sound: None,
        stories_other_sound: None,
    }
    .into()
}

pub fn dialog(peer: tl::enums::Peer, top: i32, unread: i32, pinned: bool, mute_until: Option<i32>) -> tl::types::Dialog {
    tl::types::Dialog {
        pinned,
        unread_mark: false,
        view_forum_as_messages: false,
        peer,
        top_message: top,
        read_inbox_max_id: 0,
        read_outbox_max_id: 3,
        unread_count: unread,
        unread_mentions_count: 0,
        unread_reactions_count: 0,
        unread_poll_votes_count: 0,
        notify_settings: notify(mute_until),
        pts: None,
        draft: None,
        folder_id: None,
        ttl_period: None,
    }
}

pub fn authorization(u: tl::types::User) -> tl::enums::auth::Authorization {
    tl::types::auth::Authorization {
        setup_password_required: false,
        otherwise_relogin_days: None,
        tmp_sessions: None,
        future_auth_token: None,
        user: u.into(),
    }
    .into()
}

pub fn state() -> tl::enums::updates::State {
    tl::types::updates::State { pts: 100, qts: 5, date: 1_800_000_000, seq: 1, unread_count: 0 }.into()
}

pub fn updates(updates: Vec<tl::enums::Update>, users: Vec<tl::enums::User>, chats: Vec<tl::enums::Chat>) -> tl::enums::Updates {
    tl::types::Updates { updates, users, chats, date: 0, seq: 0 }.into()
}

/// Grupo MODP de 2048 bits del RFC 3526 (primo seguro público; p ≡ 7 mod 8, válido con g = 2).
pub const RFC3526_2048: &str = concat!(
    "FFFFFFFFFFFFFFFFC90FDAA22168C234C4C6628B80DC1CD1",
    "29024E088A67CC74020BBEA63B139B22514A08798E3404DD",
    "EF9519B3CD3A431B302B0A6DF25F14374FE1356D6D51C245",
    "E485B576625E7EC6F44C42E9A637ED6B0BFF5CB6F406B7ED",
    "EE386BFB5A899FA5AE9F24117C4B1FE649286651ECE45B3D",
    "C2007CB8A163BF0598DA48361C55D39A69163FA8FD24CF5F",
    "83655D23DCA3AD961C62F356208552BB9ED529077096966D",
    "670C354E4ABC9804F1746C08CA18217C32905E462E36CE3B",
    "E39E772C180E86039B2783A2EC07A28FB5C55DF06F4C52C9",
    "DE2BCBF6955817183995497CEA956AE515D2261898FA0510",
    "15728E5A8AACAA68FFFFFFFFFFFFFFFF",
);

pub fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

pub fn password(hint: &str) -> tl::enums::account::Password {
    tl::types::account::Password {
        has_recovery: false,
        has_secure_values: false,
        has_password: true,
        current_algo: Some(
            tl::types::PasswordKdfAlgoSha256Sha256Pbkdf2Hmacsha512iter100000Sha256ModPow {
                salt1: vec![1; 8],
                salt2: vec![2; 16],
                g: 2,
                p: hex(RFC3526_2048),
            }
            .into(),
        ),
        srp_b: Some(vec![3; 256]),
        srp_id: Some(4242),
        hint: Some(hint.into()),
        email_unconfirmed_pattern: None,
        new_algo: tl::enums::PasswordKdfAlgo::Unknown,
        new_secure_algo: tl::enums::SecurePasswordKdfAlgo::Unknown,
        secure_random: vec![9; 256],
        pending_reset_date: None,
        login_email_pattern: None,
    }
    .into()
}
