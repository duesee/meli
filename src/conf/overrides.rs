/*
 * meli - conf/overrides.rs
 *
 * Copyright 2020 Manos Pitsidianakis
 *
 * This file is part of meli.
 *
 * meli is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * meli is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with meli. If not, see <http://www.gnu.org/licenses/>.
 */

#![allow(clippy::derivable_impls)]

//! This module is automatically generated by config_macros.rs.
use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PagerSettingsOverride {
    #[doc = " Number of context lines when going to next page."]
    #[doc = " Default: 0"]
    #[serde(alias = "pager-context")]
    #[serde(default)]
    pub pager_context: Option<usize>,
    #[doc = " Stop at the end instead of displaying next mail."]
    #[doc = " Default: false"]
    #[serde(alias = "pager-stop")]
    #[serde(default)]
    pub pager_stop: Option<bool>,
    #[doc = " Always show headers when scrolling."]
    #[doc = " Default: true"]
    #[serde(alias = "headers-sticky")]
    #[serde(default)]
    pub headers_sticky: Option<bool>,
    #[doc = " The height of the pager in mail view, in percent."]
    #[doc = " Default: 80"]
    #[serde(alias = "pager-ratio")]
    #[serde(default)]
    pub pager_ratio: Option<usize>,
    #[doc = " A command to pipe mail output through for viewing in pager."]
    #[doc = " Default: None"]
    #[serde(deserialize_with = "non_empty_string")]
    #[serde(default)]
    pub filter: Option<Option<String>>,
    #[doc = " A command to pipe html output before displaying it in a pager"]
    #[doc = " Default: None"]
    #[serde(deserialize_with = "non_empty_string", alias = "html-filter")]
    #[serde(default)]
    pub html_filter: Option<Option<String>>,
    #[doc = " Respect \"format=flowed\""]
    #[doc = " Default: true"]
    #[serde(alias = "format-flowed")]
    #[serde(default)]
    pub format_flowed: Option<bool>,
    #[doc = " Split long lines that would overflow on the x axis."]
    #[doc = " Default: true"]
    #[serde(alias = "split-long-lines")]
    #[serde(default)]
    pub split_long_lines: Option<bool>,
    #[doc = " Minimum text width in columns."]
    #[doc = " Default: 80"]
    #[serde(alias = "minimum-width")]
    #[serde(default)]
    pub minimum_width: Option<usize>,
    #[doc = " Choose `text/html` alternative if `text/plain` is empty in `multipart/alternative`"]
    #[doc = " attachments."]
    #[doc = " Default: true"]
    #[serde(alias = "auto-choose-multipart-alternative")]
    #[serde(default)]
    pub auto_choose_multipart_alternative: Option<ToggleFlag>,
    #[doc = " Show Date: in my timezone"]
    #[doc = " Default: true"]
    #[serde(alias = "show-date-in-my-timezone")]
    #[serde(default)]
    pub show_date_in_my_timezone: Option<ToggleFlag>,
    #[doc = " A command to launch URLs with. The URL will be given as the first argument of the command."]
    #[doc = " Default: None"]
    #[serde(deserialize_with = "non_empty_string")]
    #[serde(default)]
    pub url_launcher: Option<Option<String>>,
}
impl Default for PagerSettingsOverride {
    fn default() -> Self {
        PagerSettingsOverride {
            pager_context: None,
            pager_stop: None,
            headers_sticky: None,
            pager_ratio: None,
            filter: None,
            html_filter: None,
            format_flowed: None,
            split_long_lines: None,
            minimum_width: None,
            auto_choose_multipart_alternative: None,
            show_date_in_my_timezone: None,
            url_launcher: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ListingSettingsOverride {
    #[doc = " Number of context lines when going to next page."]
    #[doc = " Default: 0"]
    #[serde(alias = "context-lines")]
    #[serde(default)]
    pub context_lines: Option<usize>,
    #[doc = "Show auto-hiding scrollbar in accounts sidebar menu."]
    #[doc = "Default: True"]
    #[serde(default)]
    pub show_menu_scrollbar: Option<bool>,
    #[doc = " Datetime formatting passed verbatim to strftime(3)."]
    #[doc = " Default: %Y-%m-%d %T"]
    #[serde(alias = "datetime-fmt")]
    #[serde(default)]
    pub datetime_fmt: Option<Option<String>>,
    #[doc = " Show recent dates as `X {minutes,hours,days} ago`, up to 7 days."]
    #[doc = " Default: true"]
    #[serde(alias = "recent-dates")]
    #[serde(default)]
    pub recent_dates: Option<bool>,
    #[doc = " Show only envelopes that match this query"]
    #[doc = " Default: None"]
    #[serde(default)]
    pub filter: Option<Option<Query>>,
    #[serde(alias = "index-style")]
    #[serde(default)]
    pub index_style: Option<IndexStyle>,
    #[doc = "Default: \" \""]
    #[serde(default)]
    pub sidebar_mailbox_tree_has_sibling: Option<Option<String>>,
    #[doc = "Default: \" \""]
    #[serde(default)]
    pub sidebar_mailbox_tree_no_sibling: Option<Option<String>>,
    #[doc = "Default: \" \""]
    #[serde(default)]
    pub sidebar_mailbox_tree_has_sibling_leaf: Option<Option<String>>,
    #[doc = "Default: \" \""]
    #[serde(default)]
    pub sidebar_mailbox_tree_no_sibling_leaf: Option<Option<String>>,
    #[doc = "Default: ' '"]
    #[serde(default)]
    pub sidebar_divider: Option<char>,
    #[doc = "Default: 90"]
    #[serde(default)]
    pub sidebar_ratio: Option<usize>,
    #[doc = " Flag to show if thread entry contains unseen mail."]
    #[doc = " Default: \"●\""]
    #[serde(default)]
    pub unseen_flag: Option<Option<String>>,
    #[doc = " Flag to show if thread has been snoozed."]
    #[doc = " Default: \"💤\""]
    #[serde(default)]
    pub thread_snoozed_flag: Option<Option<String>>,
    #[doc = " Flag to show if thread entry has been selected."]
    #[doc = " Default: \"☑\u{fe0f}\""]
    #[serde(default)]
    pub selected_flag: Option<Option<String>>,
    #[doc = " Flag to show if thread entry contains attachments."]
    #[doc = " Default: \"📎\""]
    #[serde(default)]
    pub attachment_flag: Option<Option<String>>,
    #[doc = " Should threads with differentiating Subjects show a list of those subjects on the entry"]
    #[doc = " title?"]
    #[doc = " Default: \"true\""]
    #[serde(default)]
    pub thread_subject_pack: Option<bool>,
}
impl Default for ListingSettingsOverride {
    fn default() -> Self {
        ListingSettingsOverride {
            context_lines: None,
            show_menu_scrollbar: None,
            datetime_fmt: None,
            recent_dates: None,
            filter: None,
            index_style: None,
            sidebar_mailbox_tree_has_sibling: None,
            sidebar_mailbox_tree_no_sibling: None,
            sidebar_mailbox_tree_has_sibling_leaf: None,
            sidebar_mailbox_tree_no_sibling_leaf: None,
            sidebar_divider: None,
            sidebar_ratio: None,
            unseen_flag: None,
            thread_snoozed_flag: None,
            selected_flag: None,
            attachment_flag: None,
            thread_subject_pack: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct NotificationsSettingsOverride {
    #[doc = " Enable notifications."]
    #[doc = " Default: True"]
    #[serde(default)]
    pub enable: Option<bool>,
    #[doc = " A command to pipe notifications through"]
    #[doc = " Default: None"]
    #[serde(default)]
    pub script: Option<Option<String>>,
    #[doc = " A file location which has its size changed when new mail arrives (max 128 bytes). Can be"]
    #[doc = " used to trigger new mail notifications eg with `xbiff(1)`"]
    #[doc = " Default: None"]
    #[serde(alias = "xbiff-file-path")]
    #[serde(default)]
    pub xbiff_file_path: Option<Option<String>>,
    #[serde(alias = "play-sound")]
    #[serde(default)]
    pub play_sound: Option<ToggleFlag>,
    #[serde(alias = "sound-file")]
    #[serde(default)]
    pub sound_file: Option<Option<String>>,
}
impl Default for NotificationsSettingsOverride {
    fn default() -> Self {
        NotificationsSettingsOverride {
            enable: None,
            script: None,
            xbiff_file_path: None,
            play_sound: None,
            sound_file: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ShortcutsOverride {
    #[serde(default)]
    pub general: Option<GeneralShortcuts>,
    #[serde(default)]
    pub listing: Option<ListingShortcuts>,
    #[serde(default)]
    pub composing: Option<ComposingShortcuts>,
    #[serde(alias = "contact-list")]
    #[serde(default)]
    pub contact_list: Option<ContactListShortcuts>,
    #[serde(alias = "envelope-view")]
    #[serde(default)]
    pub envelope_view: Option<EnvelopeViewShortcuts>,
    #[serde(alias = "thread-view")]
    #[serde(default)]
    pub thread_view: Option<ThreadViewShortcuts>,
    #[serde(default)]
    pub pager: Option<PagerShortcuts>,
}
impl Default for ShortcutsOverride {
    fn default() -> Self {
        ShortcutsOverride {
            general: None,
            listing: None,
            composing: None,
            contact_list: None,
            envelope_view: None,
            thread_view: None,
            pager: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ComposingSettingsOverride {
    #[doc = " A command to pipe new emails to"]
    #[doc = " Required"]
    #[serde(default)]
    pub send_mail: Option<SendMail>,
    #[doc = " Command to launch editor. Can have arguments. Draft filename is given as the last argument. If it's missing, the environment variable $EDITOR is looked up."]
    #[serde(alias = "editor-command", alias = "editor-cmd", alias = "editor_cmd")]
    #[serde(default)]
    pub editor_command: Option<Option<String>>,
    #[doc = " Embed editor (for terminal interfaces) instead of forking and waiting."]
    #[serde(default)]
    pub embed: Option<bool>,
    #[doc = " Set \"format=flowed\" in plain text attachments."]
    #[doc = " Default: true"]
    #[serde(alias = "format-flowed")]
    #[serde(default)]
    pub format_flowed: Option<bool>,
    #[doc = "Set User-Agent"]
    #[doc = "Default: empty"]
    #[serde(alias = "insert_user_agent")]
    #[serde(default)]
    pub insert_user_agent: Option<bool>,
    #[doc = " Set default header values for new drafts"]
    #[doc = " Default: empty"]
    #[serde(alias = "default-header-values")]
    #[serde(default)]
    pub default_header_values: Option<HashMap<String, String>>,
    #[doc = " Wrap header preample when editing a draft in an editor. This allows you to write non-plain"]
    #[doc = " text email without the preamble creating syntax errors. They are stripped when you return"]
    #[doc = " from the editor. The values should be a two element array of strings, a prefix and suffix."]
    #[doc = " Default: None"]
    #[serde(alias = "wrap-header-preample")]
    #[serde(default)]
    pub wrap_header_preamble: Option<Option<(String, String)>>,
    #[doc = " Store sent mail after successful submission. This setting is meant to be disabled for"]
    #[doc = " non-standard behaviour in gmail, which auto-saves sent mail on its own."]
    #[doc = " Default: true"]
    #[serde(default)]
    pub store_sent_mail: Option<bool>,
    #[doc = " The attribution line appears above the quoted reply text."]
    #[doc = " The format specifiers for the replied address are:"]
    #[doc = " - `%+f` — the sender's name and email address."]
    #[doc = " - `%+n` — the sender's name (or email address, if no name is included)."]
    #[doc = " - `%+a` — the sender's email address."]
    #[doc = " The format string is passed to strftime(3) with the replied envelope's date."]
    #[doc = " Default: \"On %a, %0e %b %Y %H:%M, %+f wrote:%n\""]
    #[serde(default)]
    pub attribution_format_string: Option<Option<String>>,
    #[doc = " Whether the strftime call for the attribution string uses the POSIX locale instead of"]
    #[doc = " the user's active locale"]
    #[doc = " Default: true"]
    #[serde(default)]
    pub attribution_use_posix_locale: Option<bool>,
    #[doc = " Forward emails as attachment? (Alternative is inline)"]
    #[doc = " Default: ask"]
    #[serde(alias = "forward-as-attachment")]
    #[serde(default)]
    pub forward_as_attachment: Option<ToggleFlag>,
    #[doc = " Alternative lists of reply prefixes (etc. [\"Re:\", \"RE:\", ...]) to strip"]
    #[doc = " Default: `[\"Re:\", \"RE:\", \"Fwd:\", \"Fw:\", \"回复:\", \"回覆:\", \"SV:\", \"Sv:\", \"VS:\", \"Antw:\", \"Doorst:\", \"VS:\", \"VL:\", \"REF:\", \"TR:\", \"TR:\", \"AW:\", \"WG:\", \"ΑΠ:\", \"Απ:\", \"απ:\", \"ΠΡΘ:\", \"Πρθ:\", \"πρθ:\", \"ΣΧΕΤ:\", \"Σχετ:\", \"σχετ:\", \"ΠΡΘ:\", \"Πρθ:\", \"πρθ:\", \"Vá:\", \"Továbbítás:\", \"R:\", \"I:\", \"RIF:\", \"FS:\", \"BLS:\", \"TRS:\", \"VS:\", \"VB:\", \"RV:\", \"RES:\", \"Res\", \"ENC:\", \"Odp:\", \"PD:\", \"YNT:\", \"İLT:\", \"ATB:\", \"YML:\"]`"]
    #[serde(alias = "reply-prefix-list-to-strip")]
    #[serde(default)]
    pub reply_prefix_list_to_strip: Option<Option<Vec<String>>>,
    #[doc = " The prefix to use in reply subjects. The de facto prefix is \"Re:\"."]
    #[serde(alias = "reply-prefix")]
    #[serde(default)]
    pub reply_prefix: Option<String>,
}
impl Default for ComposingSettingsOverride {
    fn default() -> Self {
        ComposingSettingsOverride {
            send_mail: None,
            editor_command: None,
            embed: None,
            format_flowed: None,
            insert_user_agent: None,
            default_header_values: None,
            wrap_header_preamble: None,
            store_sent_mail: None,
            attribution_format_string: None,
            attribution_use_posix_locale: None,
            forward_as_attachment: None,
            reply_prefix_list_to_strip: None,
            reply_prefix: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TagsSettingsOverride {
    #[serde(deserialize_with = "tag_color_de")]
    #[serde(default)]
    pub colors: Option<HashMap<u64, Color>>,
    #[serde(deserialize_with = "tag_set_de", alias = "ignore-tags")]
    #[serde(default)]
    pub ignore_tags: Option<HashSet<u64>>,
}
impl Default for TagsSettingsOverride {
    fn default() -> Self {
        TagsSettingsOverride {
            colors: None,
            ignore_tags: None,
        }
    }
}

#[cfg(feature = "gpgme")]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PGPSettingsOverride {
    #[doc = " auto verify signed e-mail according to RFC3156"]
    #[doc = " Default: true"]
    #[serde(alias = "auto-verify-signatures")]
    #[serde(default)]
    pub auto_verify_signatures: Option<bool>,
    #[doc = " auto decrypt encrypted e-mail"]
    #[doc = " Default: true"]
    #[serde(alias = "auto-decrypt")]
    #[serde(default)]
    pub auto_decrypt: Option<bool>,
    #[doc = " always sign sent e-mail"]
    #[doc = " Default: false"]
    #[serde(alias = "auto-sign")]
    #[serde(default)]
    pub auto_sign: Option<bool>,
    #[doc = " Auto encrypt sent e-mail"]
    #[doc = " Default: false"]
    #[serde(alias = "auto-encrypt")]
    #[serde(default)]
    pub auto_encrypt: Option<bool>,
    #[doc = " Default: None"]
    #[serde(alias = "sign-key")]
    #[serde(default)]
    pub sign_key: Option<Option<String>>,
    #[doc = " Default: None"]
    #[serde(alias = "decrypt-key")]
    #[serde(default)]
    pub decrypt_key: Option<Option<String>>,
    #[doc = " Default: None"]
    #[serde(alias = "encrypt-key")]
    #[serde(default)]
    pub encrypt_key: Option<Option<String>>,
    #[doc = " Allow remote lookups"]
    #[doc = " Default: None"]
    #[serde(alias = "allow-remote-lookups")]
    #[serde(default)]
    pub allow_remote_lookup: Option<ToggleFlag>,
    #[doc = " Remote lookup mechanisms."]
    #[doc = " Default: \"local,wkd\""]
    #[serde(alias = "remote-lookup-mechanisms")]
    #[serde(default)]
    pub remote_lookup_mechanisms: Option<melib::gpgme::LocateKey>,
}
#[cfg(feature = "gpgme")]
impl Default for PGPSettingsOverride {
    fn default() -> Self {
        PGPSettingsOverride {
            auto_verify_signatures: None,
            auto_decrypt: None,
            auto_sign: None,
            auto_encrypt: None,
            sign_key: None,
            decrypt_key: None,
            encrypt_key: None,
            allow_remote_lookup: None,
            remote_lookup_mechanisms: None,
        }
    }
}

#[cfg(not(feature = "gpgme"))]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PGPSettingsOverride {}
#[cfg(not(feature = "gpgme"))]
impl Default for PGPSettingsOverride {
    fn default() -> Self {
        PGPSettingsOverride {}
    }
}
