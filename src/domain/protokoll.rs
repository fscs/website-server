use chrono::DateTime;
use uuid::Uuid;
use prototool::protokoll::{
    Antrag as PrototoolAntrag, Sitzung as PrototoolSitzung, SitzungKind as PrototoolSitzungKind,
    Top as PrototoolTop, TopKind as PrototoolTopKind, Event as PrototoolEvent,
};

use super::{
    antrag::Antrag, calendar::CalendarEvent, sitzung::{SitzungKind, SitzungWithTops, TopKind, TopWithAnträge}, Result, SitzungAntragPersonService
};

impl From<CalendarEvent> for PrototoolEvent {
    fn from(value: CalendarEvent) -> Self {
        Self {
            title: value.summary,
            location: value.location,
            start: value.start.unwrap_or(DateTime::from_timestamp_millis(0).unwrap()).into(),
        }
    }
}

impl From<Antrag> for PrototoolAntrag {
    fn from(value: Antrag) -> Self {
        Self {
            titel: value.data.titel,
            antragstext: value.data.antragstext,
            begründung: value.data.begründung,
        }
    }
}

impl From<TopKind> for PrototoolTopKind {
    fn from(value: TopKind) -> Self {
        match value {
            TopKind::Normal => PrototoolTopKind::Normal,
            // ewwwwwwww
            TopKind::Regularia | TopKind::Bericht | TopKind::Verschiedenes => {
                PrototoolTopKind::Verschiedenes
            }
        }
    }
}

impl From<TopWithAnträge> for PrototoolTop {
    fn from(value: TopWithAnträge) -> Self {
        Self {
            weight: value.top.weight,
            name: value.top.name,
            kind: value.top.kind.into(),
            inhalt: value.top.inhalt,
            anträge: value.anträge.into_iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<SitzungKind> for PrototoolSitzungKind {
    fn from(value: SitzungKind) -> Self {
        match value {
            SitzungKind::Normal => PrototoolSitzungKind::Normal,
            SitzungKind::VV => PrototoolSitzungKind::VV,
            SitzungKind::WahlVV => PrototoolSitzungKind::WahlVV,
            SitzungKind::Ersatz => PrototoolSitzungKind::Ersatz,
            SitzungKind::Konsti => PrototoolSitzungKind::Konsti,
            SitzungKind::Dringlichkeit => PrototoolSitzungKind::Dringlichkeit,
        }
    }
}

impl From<SitzungWithTops> for PrototoolSitzung {
    fn from(value: SitzungWithTops) -> Self {
        Self {
            id: value.sitzung.id,
            datetime: value.sitzung.datetime.into(),
            kind: value.sitzung.kind.into(),
            tops: value.tops.into_iter().map(|x| x.into()).collect(),
        }
    }
}
