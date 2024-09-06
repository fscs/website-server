use antrag_top_map::AntragTopMapRepo;
use anyhow::Result;
use chrono::{DateTime, Utc};
use person::{Abmeldung, PersonRepo};
use sitzung::{SitzungRepo, SitzungWithTops, TopWithAnträge};
use uuid::Uuid;

pub mod antrag;
pub mod antrag_top_map;
pub mod door_state;
pub mod person;
pub mod sitzung;

pub trait SitzungAntragService: SitzungRepo + AntragTopMapRepo {}

impl<T> SitzungAntragService for T where T: SitzungRepo + AntragTopMapRepo {}

pub trait SitzungPersonService: SitzungRepo + PersonRepo {}

impl<T> SitzungPersonService for T where T: SitzungRepo + PersonRepo {}

pub async fn top_with_anträge(
    repo: &mut impl SitzungAntragService,
    top_id: Uuid,
) -> Result<Option<TopWithAnträge>> {
    let Some(top) = repo.top_by_id(top_id).await? else {
        return Ok(None);
    };

    let anträge = repo.anträge_by_top(top_id).await?;

    return Ok(Some(TopWithAnträge { top, anträge }));
}

pub async fn sitzung_with_tops(
    repo: &mut impl SitzungAntragService,
    sitzung_id: Uuid,
) -> Result<Option<SitzungWithTops>> {
    let Some(sitzung) = repo.sitzung_by_id(sitzung_id).await? else {
        return Ok(None);
    };

    let tops = repo.tops_by_sitzung(sitzung_id).await?;

    let mut tops_with_anträge = vec![];

    for top in tops {
        let top_and_anträge = top_with_anträge(repo, top.id).await?.unwrap();

        tops_with_anträge.push(top_and_anträge);
    }

    return Ok(Some(SitzungWithTops {
        sitzung,
        tops: tops_with_anträge,
    }));
}

pub async fn sitzung_after_with_tops(
    repo: &mut impl SitzungAntragService,
    timestamp: DateTime<Utc>,
) -> Result<Option<SitzungWithTops>> {
    let Some(sitzung) = repo.first_sitzung_after(timestamp).await? else {
        return Ok(None);
    };

    let tops = repo.tops_by_sitzung(sitzung.id).await?;

    let mut tops_with_anträge = vec![];

    for top in tops {
        let top_and_anträge = top_with_anträge(repo, top.id).await?.unwrap();

        tops_with_anträge.push(top_and_anträge);
    }

    return Ok(Some(SitzungWithTops {
        sitzung,
        tops: tops_with_anträge,
    }));
}

pub async fn abmeldungen_by_sitzung(
    repo: &mut impl SitzungPersonService,
    sitzung_id: Uuid,
) -> Result<Option<Vec<Abmeldung>>> {
    let Some(sitzung) = repo.sitzung_by_id(sitzung_id).await? else {
        return Ok(None);
    };

    let abmeldungen = repo.abmeldungen_at(sitzung.datetime.date_naive()).await?;

    return Ok(Some(abmeldungen))
}
