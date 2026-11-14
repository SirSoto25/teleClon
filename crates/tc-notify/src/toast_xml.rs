//! XML de los toasts de Windows. Separado del backend para probarlo en cualquier SO.

use crate::{
    Notification,
    icon::{escape_xml, icon_path},
};

/// Argumento del clic en el cuerpo (`launch`).
pub(crate) const OPEN: &str = "open";
/// Prefijo de los argumentos de los botones: `a:<Action::id>`.
pub(crate) const ACTION_PREFIX: &str = "a:";

/// XML del toast. `launch` y `arguments` vuelven en `ToastActivatedEventArgs::Arguments`.
pub(crate) fn toast_xml(n: &Notification) -> String {
    let mut xml = format!(
        r#"<toast launch="{OPEN}" activationType="foreground"><visual><binding template="ToastGeneric"><text>{}</text><text>{}</text>"#,
        escape_xml(&n.title),
        escape_xml(&n.body),
    );
    if let Some(path) = n.icon.as_ref().and_then(icon_path) {
        xml.push_str(&format!(
            r#"<image placement="appLogoOverride" hint-crop="circle" src="{}"/>"#,
            escape_xml(&path.to_string_lossy()),
        ));
    }
    xml.push_str("</binding></visual>");
    if !n.actions.is_empty() {
        xml.push_str("<actions>");
        for a in &n.actions {
            xml.push_str(&format!(
                r#"<action content="{}" arguments="{ACTION_PREFIX}{}" activationType="foreground"/>"#,
                escape_xml(&a.label),
                escape_xml(&a.id),
            ));
        }
        xml.push_str("</actions>");
    }
    if n.silent {
        xml.push_str(r#"<audio silent="true"/>"#);
    } else {
        xml.push_str(r#"<audio src="ms-winsoundevent:Notification.IM"/>"#);
    }
    xml.push_str("</toast>");
    xml
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, Icon};

    fn n() -> Notification {
        Notification {
            chat_key: "u:1".into(),
            title: "Ana & <Luis>".into(),
            body: "\"hola\"".into(),
            icon: None,
            actions: vec![],
            silent: false,
        }
    }

    #[test]
    fn xml_basico_escapado_con_sonido() {
        let x = toast_xml(&n());
        assert!(x.starts_with(r#"<toast launch="open" activationType="foreground">"#));
        assert!(x.contains("<text>Ana &amp; &lt;Luis&gt;</text><text>&quot;hola&quot;</text>"));
        assert!(x.contains("ms-winsoundevent:Notification.IM"));
        assert!(!x.contains("<actions>") && !x.contains("<image"));
        assert!(x.ends_with("</toast>"));
    }

    #[test]
    fn xml_con_acciones_icono_y_silencio() {
        let mut m = n();
        m.silent = true;
        m.icon = Some(Icon::Path("C:\\a&b.png".into()));
        m.actions = vec![Action::new("mark-read", "Marcar como leído")];
        let x = toast_xml(&m);
        assert!(x.contains(r#"<audio silent="true"/>"#));
        assert!(x.contains(r#"src="C:\a&amp;b.png""#));
        assert!(x.contains(r#"<action content="Marcar como leído" arguments="a:mark-read" activationType="foreground"/>"#));
    }
}
