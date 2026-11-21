//! L'evento del calendario in corso (macOS, EventKit): dà il nome e i partecipanti a una riunione.
//! Serve il permesso Calendari, chiesto solo quando l'utente attiva l'opzione.

#[derive(Clone, Debug, Default)]
pub struct Event {
    pub title: String,
    pub attendees: Vec<String>,
}

/// "granted", "denied" o "undetermined".
pub fn access(request: bool) -> &'static str {
    imp::access(request)
}

/// L'evento iniziato da poco o che sta per iniziare (entro 10 minuti), escluso quelli di tutto il giorno.
pub fn current_event() -> Option<Event> {
    imp::current_event()
}

#[cfg(target_os = "macos")]
mod imp {
    use std::sync::mpsc;
    use std::time::Duration;

    use block2::RcBlock;
    use objc2::msg_send;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject, Bool};
    use objc2_foundation::{NSDate, NSError, NSString};

    use super::Event;

    #[link(name = "EventKit", kind = "framework")]
    extern "C" {}

    const ENTITY_EVENT: isize = 0;

    fn store_class() -> Option<&'static AnyClass> {
        AnyClass::get(c"EKEventStore")
    }

    fn status() -> isize {
        let Some(cls) = store_class() else { return 2 };
        // SAFETY: +[EKEventStore authorizationStatusForEntityType:], sola lettura.
        unsafe { msg_send![cls, authorizationStatusForEntityType: ENTITY_EVENT] }
    }

    pub fn access(request: bool) -> &'static str {
        // 0 da decidere, 1 limitato, 2 negato, 3 concesso (pieno), 4 solo scrittura
        match status() {
            3 => return "granted",
            0 if request => {}
            0 => return "undetermined",
            _ => return "denied",
        }
        let Some(cls) = store_class() else {
            return "denied";
        };
        let (tx, rx) = mpsc::channel();
        let done = RcBlock::new(move |granted: Bool, _e: *mut NSError| {
            let _ = tx.send(granted.as_bool());
        });
        // SAFETY: richiesta d'accesso con un blocco di completamento che vive fino alla risposta.
        unsafe {
            let store: Retained<AnyObject> = msg_send![cls, new];
            let sel = objc2::sel!(requestFullAccessToEventsWithCompletion:);
            let modern: bool = msg_send![&*store, respondsToSelector: sel];
            if modern {
                let () = msg_send![&*store, requestFullAccessToEventsWithCompletion: &*done];
            } else {
                let () =
                    msg_send![&*store, requestAccessToEntityType: ENTITY_EVENT, completion: &*done];
            }
            // la finestra di sistema aspetta l'utente: fino a 2 minuti
            match rx.recv_timeout(Duration::from_secs(120)) {
                Ok(true) => "granted",
                _ => "denied",
            }
        }
    }

    fn string(obj: &AnyObject, sel_name: &str) -> String {
        // SAFETY: proprietà NSString di sola lettura (title, name).
        let s: Option<Retained<NSString>> = unsafe {
            match sel_name {
                "title" => msg_send![obj, title],
                _ => msg_send![obj, name],
            }
        };
        s.map(|s| s.to_string()).unwrap_or_default()
    }

    pub fn current_event() -> Option<Event> {
        if status() != 3 {
            return None;
        }
        let cls = store_class()?;
        // SAFETY: EKEventStore, predicato su un intervallo, lettura di eventi e partecipanti.
        unsafe {
            let store: Retained<AnyObject> = msg_send![cls, new];
            let from = NSDate::dateWithTimeIntervalSinceNow(-60.0 * 60.0 * 4.0);
            let to = NSDate::dateWithTimeIntervalSinceNow(60.0 * 10.0);
            let pred: Retained<AnyObject> = msg_send![&*store, predicateForEventsWithStartDate: &*from, endDate: &*to, calendars: std::ptr::null::<AnyObject>()];
            let events: Retained<AnyObject> = msg_send![&*store, eventsMatchingPredicate: &*pred];
            let count: usize = msg_send![&*events, count];
            let now = NSDate::now();
            let soon = NSDate::dateWithTimeIntervalSinceNow(60.0 * 10.0);
            let mut best: Option<(f64, Event)> = None;
            for i in 0..count {
                let ev: Retained<AnyObject> = msg_send![&*events, objectAtIndex: i];
                let all_day: bool = msg_send![&*ev, isAllDay];
                if all_day {
                    continue;
                }
                let start: Retained<NSDate> = msg_send![&*ev, startDate];
                let end: Retained<NSDate> = msg_send![&*ev, endDate];
                // in corso, o che inizia entro 10 minuti
                let running = start.timeIntervalSinceDate(&soon) <= 0.0
                    && end.timeIntervalSinceDate(&now) > 0.0;
                if !running {
                    continue;
                }
                let title = string(&ev, "title");
                let mut attendees = vec![];
                let list: Option<Retained<AnyObject>> = msg_send![&*ev, attendees];
                if let Some(list) = list {
                    let n: usize = msg_send![&*list, count];
                    for k in 0..n {
                        let who: Retained<AnyObject> = msg_send![&*list, objectAtIndex: k];
                        let name = string(&who, "name");
                        if !name.is_empty() {
                            attendees.push(name);
                        }
                    }
                }
                // tra più eventi sovrapposti vince quello iniziato più di recente
                let age = now.timeIntervalSinceDate(&start).abs();
                if best.as_ref().is_none_or(|(a, _)| age < *a) {
                    best = Some((age, Event { title, attendees }));
                }
            }
            best.map(|(_, e)| e)
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::Event;
    pub fn access(_request: bool) -> &'static str {
        "denied"
    }
    pub fn current_event() -> Option<Event> {
        None
    }
}

#[cfg(test)]
mod tests {
    /// Sul Mac: `cargo test --lib calendar_reads_this_mac -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn calendar_reads_this_mac() {
        println!(
            "accesso: {}, evento: {:?}",
            super::access(false),
            super::current_event()
        );
    }
}
