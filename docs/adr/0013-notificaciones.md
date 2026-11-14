# ADR-0013: Notificaciones de escritorio (crate `tc-notify`)

- **Fecha:** 2026-11-14 · **Estado:** aceptado (fases C1 y C3 del plan `docs/plans/archivos-busqueda-notificaciones.md`).
  La integración en `tc-ui`/`tc-app` (C2) es posterior. Probado en Linux real (CachyOS, sesión gráfica con D-Bus); Windows y
  macOS solo compilan (`cargo check` en cruzado).
- **Contexto:** Parlo no avisa de mensajes nuevos. Es la queja más repetida de Telegram Desktop en el informe de
  emprendedor (`replica/entrepreneur-report.md`: #91 notificaciones nativas en Linux, #277 y #24556 control y
  ruido, #7529 silenciar por carpeta, «X se unió a Telegram»). Hace falta un aviso nativo en los tres SO, con
  clic que abra el chat, y reglas que eviten el ruido (ráfagas al reconectar, chats silenciados) y protejan la
  privacidad (texto en la pantalla de bloqueo, chats secretos).

## Decisiones

1. **Crate propio `tc-notify`, sin depender de `tc-core`.** Las reglas reciben structs propios (`ChatInfo`,
   `IncomingMessage`, `WindowState`, `NotifySettings`) y la clave de chat es una cadena opaca (`ChatKey`). Así el
   crate no se rompe con cambios del modelo, se prueba sin backend y la traducción vive en un único sitio (C2).
2. **Interfaz `Notifier: Send + Sync`** con `notify(Notification)`, `withdraw(&ChatKey)` y `capabilities()`.
   Los eventos de vuelta (`NotifyEvent::Clicked(chat)`, `NotifyEvent::Action(chat, id)`) llegan por un
   `futures::channel::mpsc::UnboundedReceiver` (`NotifyEvents`), que iced consume como `Subscription` sin hilos
   extra. `system(&AppInfo)` elige el backend del SO y, si no hay servicio (sin sesión D-Bus, por ejemplo),
   devuelve `NullNotifier` con un `warn!`: la app nunca falla por no poder avisar.
3. **Un aviso por chat.** Un aviso nuevo del mismo chat sustituye al anterior (Linux `replaces_id`, Windows
   `Tag`+`Group`), con el recuento «Ana (3)» que calcula `rules::Engine`. Al leer el chat, `Engine::on_chat_read`
   devuelve `true` y la app llama a `withdraw`.
4. **Linux/BSD: `zbus` 5 directo**, no `notify-rust`.
   - Rust puro, sin `libdbus` ni cabeceras: no añade dependencias de sistema a la CI. `zbus` 5.19 ya estaba en
     el árbol por el llavero (Secret Service), así que no añade crates.
   - `notify-rust` (que también usa zbus) solo da las acciones con `wait_for_action`, que bloquea un hilo por
     aviso hasta que se cierra. Con zbus hay una conexión y dos hilos fijos que escuchan `ActionInvoked` y
     `NotificationClosed` para todos los avisos.
   - Pistas: `desktop-entry=parlo` (el servidor toma nombre e icono de `parlo.desktop` y GNOME lo lista en sus
     ajustes), `category=im.received`, `sound-name=message-new-instant` o `suppress-sound`, `image-path` para la
     foto del chat. Si el servidor anuncia `body-markup`, el cuerpo se escapa. Las acciones solo se envían si
     anuncia `actions`; el clic en el cuerpo es la acción `default`.
   - **No** se usa el portal XDG de notificaciones (`org.freedesktop.portal.Notification`): en una app sin
     sandbox el portal no aporta nada y sus acciones activan la app por D-Bus (necesita servicio D-Bus
     activable). Se reconsiderará con Flatpak.
5. **Windows: crate oficial `windows` 0.62** (bindings Rust de WinRT, sin C), no `tauri-winrt-notification`.
   Ese crate no permite fijar `Tag`/`Group` (sin sustitución por chat ni retirada) y añade `windows-version`;
   con `windows` se escribe el XML del toast (`toast_xml.rs`, probado en cualquier SO), se fijan `Tag` (hash de
   16 caracteres, límite de Windows 10 antiguos) y `Group`, se escucha `Activated` (cuerpo = `launch="open"`,
   botones = `arguments="a:<id>"`) y se retira con `History().RemoveGroupedTagWithId`. `windows` 0.62 ya
   estaba en el `Cargo.lock`.
   - **AppUserModelID `app.parlo.Parlo`**: Windows solo muestra toasts de una app de escritorio sin MSIX si un
     acceso directo del menú Inicio lleva ese AUMID. Lo pone el instalador Inno (`[Icons] AppUserModelID`), y el
     proceso declara el mismo con `SetCurrentProcessExplicitAppUserModelID`. Un manifiesto Win32 no puede
     declararlo (solo MSIX); se deja comentado en `parlo.manifest`. El zip portable no crea acceso directo: sin
     instalador no hay toasts.
   - Sin activador COM (`ToastActivatorCLSID`): el clic llega mientras Parlo está abierto. Un aviso antiguo del
     centro de actividades con Parlo cerrado solo abre la app por el acceso directo. Suficiente para C2.
6. **macOS: `mac-notification-sys` 0.6** (`NSUserNotification`), no `objc2-user-notifications`
   (`UNUserNotificationCenter`).
   - `UNUserNotificationCenter` aborta si el proceso no está en un `.app` firmado: rompería `cargo run` y las
     pruebas de desarrollo. `NSUserNotification` funciona con el binario suelto y dentro de `Parlo.app`; está
     obsoleta desde macOS 11 pero sigue funcionando en macOS 15. Es la misma base que usa `notify-rust`.
   - Coste: compila un `.m` con clang de Xcode (solo en macOS; la CI de macOS ya tiene Xcode). Cada aviso espera
     su respuesta en un hilo propio (la biblioteca bloquea). Sin sustitución por chat ni retirada (no expone el
     id): `capabilities()` lo dice y la app no depende de ello. Los botones se ven con el estilo «alerta»
     (`NSUserNotificationAlertStyle=alert` en `Info.plist`).
   - Migrar a `UNUserNotificationCenter` cuando Parlo se distribuya siempre firmado y notarizado.
7. **Reglas (`tc_notify::rules`)**, puras y sin E/S; horas en segundos Unix:
   - `check` en este orden: mensaje propio → nunca; interruptor general; «No molestar» (`Off`/`On`/`Until`, gana a
     las menciones); «se unió a Telegram» (ignorado por defecto); ventana con foco **y** ese chat abierto;
     desactivado por tipo (privados+secretos, grupos, canales); archivado no silenciado (ajuste, por defecto sí
     avisa, como Telegram); silenciado (`mute_until` manda sobre `muted`: vencido el plazo vuelve a sonar) salvo
     mención con `mentions_bypass_mute`.
   - `render`: vista previa `Full` (en grupos «Autor: texto», recortado a 200 caracteres), `NameOnly` (nombre del
     chat y «N mensajes nuevos») u `Hidden` («Parlo» y «Mensaje nuevo»). **Chats secretos**: la vista previa
     efectiva es la menor de la general y `secret_preview`, que por defecto es `NameOnly`: nunca texto salvo que
     se pida.
   - `Engine` (con estado): agrupa por chat (recuento desde la última lectura), limita ráfagas (por defecto 5
     avisos en 10 s entre todos los chats; lo que se pasa del límite se cuenta y sale en el siguiente aviso de
     ese chat) y suena como mucho una vez por ventana; respeta el `silent` del remitente.
   - Las cadenas visibles están en español en `rules`; se moverán a la capa de idioma cuando exista.

## Consecuencias
- Linux sin servidor de notificaciones o sin sesión D-Bus: `NullNotifier`, sin error visible.
- Wayland: el clic llega como evento, pero enfocar la ventana desde fuera depende del compositor (sin token
  `xdg-activation`, GNOME puede limitarse a resaltar la ventana). C2 lo comprobará; el token
  (`ActivationToken`, spec 1.2) se puede añadir sin cambiar la interfaz.
- Una tormenta al reconectar produce como mucho `burst.max` avisos y un sonido.
- Dependencias nuevas en `Cargo.toml` del workspace: `zbus`, `windows` y `mac-notification-sys`, cada una solo
  en su SO. En Linux no cambia el árbol de crates compilados salvo `tc-notify`.

## Verificación (2026-11-14)
- Linux: `cargo test -p tc-notify` (43 pruebas; 1 ignorada) y la ignorada `aviso_real` contra el servidor de
  notificaciones real (mostrar, sustituir el aviso del mismo chat y cerrarlo). Clic y botones **no** probados (hace
  falta pulsar a mano).
- `cargo +stable check -p tc-notify --all-targets` para `x86_64-pc-windows-msvc` y `aarch64-apple-darwin` con
  `RUSTFLAGS=-D warnings` (en macOS con un compilador C ficticio: el `.m` no se compila de verdad). Sin ejecutar en
  Windows ni macOS.
