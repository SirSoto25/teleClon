//! Menús contextuales de chats y mensajes: ítems según el tipo de chat, estado optimista con
//! reversión, confirmaciones, «Deshacer», atajos, selección múltiple y eventos de borrado. Como en
//! el resto de pruebas, las `Task` no se ejecutan: la prueba entrega los resultados como el runtime.

use std::sync::Arc;

use iced::keyboard::{self, Key, Modifiers, key::Named};
use tc_core::*;
use tc_mock::{BIG_CHAT, MockBackend, MockConfig};

use crate::app::chat_actions::{ChatMsg, Confirm, MenuTarget, MuteFor, action_error_text, message_link};
use crate::app::{App, Backend, Flags, Msg};
use crate::view::context_menu::{chat_items, message_items};

const NOW: i64 = 1_762_430_400;

async fn boot() -> (App, Arc<MockBackend>) {
    let mock = Arc::new(MockBackend::new(MockConfig { now: Some(NOW), ..Default::default() }));
    let backend: Backend = mock.clone();
    let (mut app, _) = App::new(Flags::new(backend.clone()));
    let _ = app.update(Msg::Me(backend.me().await));
    let _ = app.update(Msg::DialogsLoaded(backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await));
    for d in app.dialogs.clone() {
        if let PeerRef::Group(g) = d.peer {
            let _ = app.update(Msg::GroupLoaded(backend.group(g).await));
        }
    }
    (app, mock)
}

fn labels(items: &[crate::view::context_menu::Item]) -> Vec<String> {
    items.iter().filter_map(|i| i.label().map(str::to_string)).collect()
}

fn chat_msg(m: ChatMsg) -> Msg {
    Msg::Chat(m)
}

fn a_user(app: &App) -> PeerRef {
    app.dialogs.iter().find(|d| matches!(d.peer, PeerRef::User(_)) && d.peer != BIG_CHAT && !app.is_saved(d.peer)).unwrap().peer
}

fn key(k: Key) -> Msg {
    Msg::Key(keyboard::Event::KeyPressed {
        key: k.clone(),
        modified_key: k,
        physical_key: keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers: Modifiers::default(),
        text: None,
        repeat: false,
    })
}

#[tokio::test]
async fn chat_menu_items_depend_on_the_kind_of_chat() {
    let (mut app, _) = boot().await;
    let user = a_user(&app);
    let l = labels(&chat_items(&app, user, false));
    for want in ["Fijar arriba", "Silenciar…", "Archivar", "Ver información", "Vaciar historial", "Bloquear usuario", "Borrar chat"] {
        assert!(l.iter().any(|x| x == want), "falta «{want}» en {l:?}");
    }
    assert!(!l.iter().any(|x| x.starts_with("Salir")));
    let channel = app.dialogs.iter().find(|d| app.is_channel(d.peer)).unwrap().peer;
    let l = labels(&chat_items(&app, channel, false));
    assert!(l.contains(&"Salir del canal".to_string()) && !l.contains(&"Vaciar historial".to_string()) && !l.contains(&"Borrar chat".to_string()));
    let secret = PeerRef::Secret(tc_mock::SECRET_ID);
    let l = labels(&chat_items(&app, secret, false));
    assert!(l.contains(&"Borrar chat secreto".to_string()) && !l.contains(&"Archivar".to_string()));
    // Submenú de silencio y el menú abierto con clic derecho.
    assert_eq!(labels(&chat_items(&app, user, true))[..4], MuteFor::ALL.map(|f| f.label().to_string()));
    let _ = app.update(chat_msg(ChatMsg::OpenChatMenu(user)));
    assert_eq!(app.chat_actions.menu.map(|m| m.target), Some(MenuTarget::Chat(user)));
    let _ = app.update(chat_msg(ChatMsg::MuteSubmenu));
    assert!(app.chat_actions.menu.unwrap().mute_sub);
    // Elegir una acción cierra el menú.
    let _ = app.update(chat_msg(ChatMsg::Mute(user, Some(MuteFor::Hour))));
    assert!(app.chat_actions.menu.is_none() && app.dialog(user).unwrap().muted);
}

#[tokio::test]
async fn archive_is_optimistic_reverts_on_error_and_can_be_undone() {
    let (mut app, _) = boot().await;
    let peer = a_user(&app);
    let _ = app.update(chat_msg(ChatMsg::Archive(peer, true)));
    assert!(app.dialog(peer).unwrap().archived);
    assert!(app.visible_dialogs().iter().all(|d| d.peer != peer), "archivado: fuera de la lista principal");
    let prev = {
        let mut d = app.dialog(peer).unwrap().clone();
        d.archived = false;
        Some(Box::new(d))
    };
    // La red falla: vuelve a su sitio con un aviso claro.
    let _ = app.update(chat_msg(ChatMsg::Done { peer, prev: prev.clone(), ok: None, result: Err(BackendError::Network) }));
    assert!(!app.dialog(peer).unwrap().archived);
    assert!(app.toasts.iter().any(|t| t.error));
    // Sale bien: aviso con «Deshacer».
    let _ = app.update(chat_msg(ChatMsg::Archive(peer, true)));
    let _ = app.update(chat_msg(ChatMsg::Done { peer, prev, ok: Some(("Chat archivado.".into(), Some(Box::new(ChatMsg::Archive(peer, false))))), result: Ok(()) }));
    assert_eq!(app.chat_actions.undo.as_ref().map(|u| u.text.as_str()), Some("Chat archivado."));
    let _ = app.update(chat_msg(ChatMsg::Undo));
    assert!(app.chat_actions.undo.is_none() && !app.dialog(peer).unwrap().archived);
    // La fila «Archivados» lleva a la lista de archivados.
    let _ = app.update(chat_msg(ChatMsg::Archive(peer, true)));
    let _ = app.update(chat_msg(ChatMsg::ToggleArchived));
    assert!(app.visible_dialogs().iter().any(|d| d.peer == peer));
}

#[tokio::test]
async fn unread_mark_read_pin_limit_message() {
    let (mut app, _) = boot().await;
    let peer = a_user(&app);
    let _ = app.update(chat_msg(ChatMsg::MarkUnread(peer)));
    assert!(app.dialog(peer).unwrap().looks_unread());
    assert!(labels(&chat_items(&app, peer, false)).contains(&"Marcar como leído".to_string()));
    let _ = app.update(chat_msg(ChatMsg::MarkRead(peer)));
    let d = app.dialog(peer).unwrap();
    assert!(!d.unread_mark && d.unread_count == 0);
    // Límite de fijados: se revierte y se explica.
    let prev = Some(Box::new(app.dialog(peer).unwrap().clone()));
    let _ = app.update(chat_msg(ChatMsg::Pin(peer, true)));
    assert!(app.dialog(peer).unwrap().pinned_order.is_some());
    let _ = app.update(chat_msg(ChatMsg::Done { peer, prev, ok: None, result: Err(BackendError::Forbidden("PINNED_DIALOGS_TOO_MUCH".into())) }));
    assert!(app.dialog(peer).unwrap().pinned_order.is_none());
    assert!(app.toasts.last().unwrap().text.contains("máximo de chats fijados"));
}

#[tokio::test]
async fn delete_key_asks_then_removes_and_restores_on_error() {
    let (mut app, mock) = boot().await;
    let peer = a_user(&app);
    let _ = app.update(Msg::Open(peer));
    let _ = app.update(key(Key::Named(Named::Delete)));
    assert!(matches!(app.chat_actions.confirm, Some(Confirm::DeleteChat { peer: p, revoke: false, can_revoke: true }) if p == peer));
    let _ = app.update(chat_msg(ChatMsg::SetRevoke(true)));
    // Escape cancela; Supr otra vez y Enter confirma.
    let _ = app.update(key(Key::Named(Named::Escape)));
    assert!(app.chat_actions.confirm.is_none() && app.conv.is_some());
    let _ = app.update(key(Key::Named(Named::Delete)));
    let removed = app.dialog(peer).cloned();
    let _ = app.update(key(Key::Named(Named::Enter)));
    assert!(app.dialog(peer).is_none() && app.conv.is_none(), "se quita al momento");
    let _ = app.update(chat_msg(ChatMsg::Removed { prev: removed.map(Box::new), ok: "Chat borrado.".into(), result: Err(BackendError::Network) }));
    assert!(app.dialog(peer).is_some(), "error de red: la fila vuelve");
    // Contra el mock de verdad: el evento quita la fila.
    mock.delete_dialog(peer, true).await.unwrap();
    let _ = app.update(Msg::Backend(BackendEvent::DialogRemoved { peer }));
    assert!(app.dialog(peer).is_none());
}

#[tokio::test]
async fn leave_block_info_and_clear() {
    let (mut app, mock) = boot().await;
    let group = app.dialogs.iter().find(|d| matches!(d.peer, PeerRef::Group(_)) && !app.is_channel(d.peer)).unwrap().peer;
    let _ = app.update(Msg::Open(group));
    let _ = app.update(key(Key::Named(Named::Delete)));
    assert_eq!(app.chat_actions.confirm, Some(Confirm::Leave { peer: group }));
    let _ = app.update(chat_msg(ChatMsg::CancelConfirm));
    // Bloquear: confirmación, estado conocido y «Deshacer».
    let user = a_user(&app);
    let PeerRef::User(u) = user else { unreachable!() };
    let _ = app.update(chat_msg(ChatMsg::AskBlock(user)));
    let _ = app.update(chat_msg(ChatMsg::Confirm));
    assert_eq!(app.chat_actions.blocked.get(&u), Some(&true));
    assert!(labels(&chat_items(&app, user, false)).contains(&"Desbloquear".to_string()));
    let _ = app.update(chat_msg(ChatMsg::BlockDone { peer: user, user: u, blocked: true, result: Ok(()) }));
    assert!(app.chat_actions.undo.as_ref().unwrap().text.contains("bloqueado"));
    // Información.
    let _ = app.update(chat_msg(ChatMsg::ShowInfo(group)));
    assert!(app.chat_actions.info.as_ref().unwrap().info.is_none());
    let _ = app.update(chat_msg(ChatMsg::InfoLoaded(group, mock.chat_info(group).await)));
    assert!(app.chat_actions.info.as_ref().unwrap().info.as_ref().unwrap().as_ref().unwrap().members.is_some());
    let _ = app.update(key(Key::Named(Named::Escape)));
    assert!(app.chat_actions.info.is_none());
    // Vaciar historial del chat abierto.
    let _ = app.update(Msg::History { peer: group, older: false, result: mock.history(group, Anchor::Latest, 50).await });
    assert!(!app.conv.as_ref().unwrap().messages.is_empty());
    let _ = app.update(Msg::Backend(BackendEvent::HistoryCleared { peer: group }));
    assert!(app.conv.as_ref().unwrap().messages.is_empty());
}

#[tokio::test]
async fn message_menu_selection_and_bulk_delete() {
    let (mut app, mock) = boot().await;
    let peer = a_user(&app);
    let _ = app.update(Msg::Open(peer));
    let page = mock.history(peer, Anchor::Latest, 50).await.unwrap();
    let _ = app.update(Msg::History { peer, older: false, result: Ok(page.clone()) });
    let own = page.iter().rev().find(|m| m.outgoing && !m.text.is_empty()).map(|m| m.id);
    let theirs = page.iter().rev().find(|m| !m.outgoing).unwrap().id;
    let _ = app.update(chat_msg(ChatMsg::OpenMessageMenu(theirs)));
    assert_eq!(app.chat_actions.menu.map(|m| m.target), Some(MenuTarget::Message(theirs)));
    let l = labels(&message_items(&app, theirs));
    for want in ["Responder", "Reenviar", "Fijar mensaje", "Seleccionar", "Borrar"] {
        assert!(l.iter().any(|x| x == want), "falta «{want}» en {l:?}");
    }
    assert!(!l.contains(&"Editar".to_string()), "no se edita lo ajeno");
    if let Some(own) = own {
        // Los propios del mock son antiguos (más de 48 h): fuera de plazo para editar.
        let fresh = app.conv.as_ref().unwrap().find(own).is_some_and(|m| NOW - m.date.0 < crate::app::chat_actions::EDIT_WINDOW_SECS);
        assert_eq!(labels(&message_items(&app, own)).contains(&"Editar".to_string()), fresh && app.can_edit(app.conv.as_ref().unwrap().find(own).unwrap()));
    }
    // Fijar: optimista y revertido si falla.
    let _ = app.update(chat_msg(ChatMsg::PinMessage(theirs, true)));
    assert!(app.chat_actions.msg_pinned(peer, theirs));
    let _ = app.update(chat_msg(ChatMsg::PinnedMessage { peer, id: theirs, pinned: true, result: Err(BackendError::Network) }));
    assert!(!app.chat_actions.msg_pinned(peer, theirs));
    // Seleccionar varios y borrar en bloque (con confirmación).
    let _ = app.update(chat_msg(ChatMsg::StartSelect(theirs)));
    let other = page.iter().rev().filter(|m| m.id != theirs).nth(1).unwrap().id;
    let _ = app.update(chat_msg(ChatMsg::ToggleSelect(other)));
    assert_eq!(app.chat_actions.selection.as_ref().unwrap().len(), 2);
    let _ = app.update(key(Key::Named(Named::Delete)));
    assert!(matches!(&app.chat_actions.confirm, Some(Confirm::DeleteMessages { ids, .. }) if ids.len() == 2));
    let _ = app.update(chat_msg(ChatMsg::Confirm));
    assert!(app.chat_actions.selection.is_none() && app.chat_actions.confirm.is_none());
    // Reenviar: selector y confirmación.
    let _ = app.update(chat_msg(ChatMsg::Forward(vec![theirs])));
    let to = app.forward_targets("").into_iter().find(|d| d.peer != peer).unwrap().peer;
    let _ = app.update(chat_msg(ChatMsg::ForwardTo(to)));
    assert!(app.chat_actions.forward.as_ref().unwrap().busy);
    let _ = app.update(chat_msg(ChatMsg::Forwarded { to, n: 1, result: Ok(()) }));
    assert!(app.chat_actions.forward.is_none() && app.toasts.last().unwrap().text.starts_with("Mensaje reenviado"));
}

#[test]
fn errors_and_links_in_spanish() {
    assert!(action_error_text(&BackendError::Forbidden("USER_CREATOR".into())).contains("transfiere"));
    assert!(action_error_text(&BackendError::Forbidden(UNSUPPORTED.into())).contains("no está disponible"));
    assert_eq!(action_error_text(&BackendError::Network), crate::app::error_text(&BackendError::Network));
    let mut g = Group { id: GroupId(-1_000_000_000_042), kind: GroupKind::Super, title: "x".into(), username: None, member_count: 3, photo: None, is_forum: false };
    assert_eq!(message_link(&g, MsgId(7)).as_deref(), Some("https://t.me/c/42/7"));
    g.username = Some("charla".into());
    assert_eq!(message_link(&g, MsgId(7)).as_deref(), Some("https://t.me/charla/7"));
    g.kind = GroupKind::Basic;
    assert_eq!(message_link(&g, MsgId(7)), None);
    assert_eq!(MuteFor::Hour.until(Timestamp(100)), Some(Timestamp(3700)));
    assert_eq!(MuteFor::Forever.until(Timestamp(100)), None);
}

#[tokio::test]
async fn a_chat_never_appears_twice_after_reloading_the_lists() {
    let (mut app, mock) = boot().await;
    let backend: Backend = mock.clone();
    let user = a_user(&app);
    // Copia archivada obsoleta del mismo chat (p. ej. restos de la caché) y repetido en la propia lista.
    let mut stale = app.dialogs.iter().find(|d| d.peer == user).unwrap().clone();
    stale.archived = true;
    app.dialogs.push(stale.clone());
    let mut list = backend.dialogs(DialogsPage { offset: 0, limit: 500, archived: false }).await.unwrap();
    list.push(list[0].clone());
    let _ = app.update(Msg::DialogsLoaded(Ok(list)));
    let count = |app: &App, p: PeerRef| app.dialogs.iter().filter(|d| d.peer == p).count();
    assert!(app.dialogs.iter().all(|d| count(&app, d.peer) == 1), "hay chats repetidos");
    // Y un archivado que Telegram ya no da como archivado desaparece de la carpeta.
    app.dialogs.push(stale);
    let _ = app.update(Msg::Chat(ChatMsg::ArchivedLoaded(Ok(vec![]))));
    assert!(app.dialogs.iter().all(|d| !d.archived || d.peer != user));
}
