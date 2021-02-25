use crate::mail::send::EmailSender;
#[cfg(all(not(test), not(feature = "local")))]
use crate::mail::send::SesSender;
use crate::mail::templates::Template;
use crate::mail::templates::TemplateManager;
use crate::mail::Email;
#[cfg(all(not(test), not(feature = "local")))]
use crate::settings::Settings;
use actix_rt::Arbiter;
use basket::Basket;
use basket::SubscribeOpts;
use basket::YesNo;
#[cfg(all(not(test), not(feature = "local")))]
use lazy_static::lazy_static;
use log::error;

const MOZILLIAN_NDA_LIST: &str = "mozillians-nda";

#[cfg(all(not(test), not(feature = "local")))]
lazy_static! {
    static ref MAIL_MAN: MailMan<SesSender> = {
        let s = Settings::new().expect("invalid settings");
        let basket = s.basket.map(|b| Basket::new(b.api_key, b.basket_url));
        MailMan::<SesSender>::new(s.packs.domain, s.packs.catcher, basket)
    };
}

#[cfg(all(not(test), not(feature = "local")))]
pub fn send_email(to: String, t: &Template) {
    let message = MAIL_MAN.template_man.render(t);
    MAIL_MAN.send(Email::with(to, &MAIL_MAN.template_man.domain, message));
}

#[cfg(all(not(test), not(feature = "local")))]
pub fn send_emails(to: Vec<String>, t: &Template) {
    let message = MAIL_MAN.template_man.render(t);
    MAIL_MAN.send(Email::with_many(to, &MAIL_MAN.template_man.domain, message));
}

#[cfg(all(not(test), not(feature = "local")))]
pub fn send_email_raw(mut email: Email) {
    email.from = format!("no-reply@{}", &MAIL_MAN.template_man.domain);
    MAIL_MAN.send(email);
}

#[cfg(any(test, feature = "local"))]
pub fn send_email(_: String, _: &Template) {}

#[cfg(any(test, feature = "local"))]
pub fn send_emails(_: Vec<String>, _: &Template) {}

#[cfg(any(test, feature = "local"))]
pub fn send_email_raw(_: Email) {}

#[cfg(all(not(test), not(feature = "local")))]
pub fn subscribe_nda(email: impl Into<String>) {
    MAIL_MAN.subscribe_nda(email);
}

#[cfg(all(not(test), not(feature = "local")))]
pub fn unsubscribe_nda(email: impl Into<String>) {
    MAIL_MAN.unsubscribe_nda(email);
}

#[cfg(any(test, feature = "local"))]
pub fn subscribe_nda(_: impl Into<String>) {}

#[cfg(any(test, feature = "local"))]
pub fn unsubscribe_nda(_: impl Into<String>) {}

pub struct MailMan<T: EmailSender> {
    pub arbiter: Arbiter,
    pub sender: T,
    pub template_man: TemplateManager,
    pub catcher: Option<String>,
    pub basket: Option<Basket>,
}

impl<T: EmailSender> MailMan<T> {
    pub fn new(domain: String, catcher: Option<String>, basket: Option<Basket>) -> Self {
        MailMan {
            arbiter: Arbiter::new(),
            sender: T::default(),
            template_man: TemplateManager::new(domain),
            catcher,
            basket,
        }
    }
}

impl<T: EmailSender> MailMan<T> {
    pub fn send(&self, mut e: Email) {
        if let Some(ref catcher) = self.catcher {
            if let Some(to) = e.to {
                e.message.body = format!("[to: caught for {}]\n\n{}", to, e.message.body);
                e.to = Some(catcher.to_owned());
            };
            if let Some(bcc) = e.bcc {
                e.message.body =
                    format!("[bcc: caught for {}]\n\n{}", bcc.join(", "), e.message.body);
                e.to = Some(catcher.to_owned());
                e.bcc = None;
            };
        }
        let s = self.sender.clone();
        let f = Box::pin(async move {
            if let Err(e) = s.send_email(e).await {
                error!("Error sending email: {}", e);
            }
        });
        self.arbiter.spawn(f);
    }

    pub fn subscribe_nda(&self, email: impl Into<String>) {
        if let Some(basket) = self.basket.clone() {
            let email = email.into();
            let opts = SubscribeOpts {
                source_url: Some(self.template_man.domain.clone()),
                optin: Some(YesNo::Y),
                ..Default::default()
            };
            let f = Box::pin(async move {
                if let Err(e) = basket
                    .subscribe_private(&email, vec![MOZILLIAN_NDA_LIST.into()], Some(opts))
                    .await
                {
                    error!("Error subscribing {} to nda-list: {}", email, e);
                }
            });
            self.arbiter.spawn(f);
        }
    }

    pub fn unsubscribe_nda(&self, email: impl Into<String>) {
        if let Some(basket) = self.basket.clone() {
            let email = email.into();
            let f = Box::pin(async move {
                let token = match basket.lookup_user(&email).await {
                    Ok(j) if j["token"].is_string() => {
                        j["token"].as_str().unwrap_or_default().to_owned()
                    }
                    Ok(_) => {
                        error!("Invalid JSON when retrieving token for {}", &email);
                        return;
                    }
                    Err(e) => {
                        error!("Unable to retrieve token for {}: {}", &email, e);
                        return;
                    }
                };
                if let Err(e) = basket
                    .unsubscribe(&token, vec![MOZILLIAN_NDA_LIST.into()], YesNo::N)
                    .await
                {
                    error!("Error unsubscribing {} to nda-list: {}", &email, e);
                }
            });
            self.arbiter.spawn(f);
        }
    }
}
