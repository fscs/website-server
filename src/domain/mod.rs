use antrag_top_map::AntragTopMapRepo;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod antrag;
pub mod antrag_top_map;
pub mod calendar;
pub mod door_state;
pub mod persons;
pub mod sitzung;

use persons::{Abmeldung, PersonRepo};
use sitzung::{SitzungRepo, SitzungWithTops, SitzungenWithTops, TopWithAnträge};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database returned an error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("request error")]
    Reqwest(#[from] reqwest::Error),
    #[error("{0}")]
    Message(String),
}

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

    Ok(Some(TopWithAnträge { top, anträge }))
}

pub async fn top_with_anträge_by_sitzung(
    repo: &mut impl SitzungAntragService,
    sitzung_id: Uuid,
) -> Result<Vec<TopWithAnträge>> {
    let tops = repo.tops_by_sitzung(sitzung_id).await?;

    let mut tops_with_anträge = vec![];

    for top in tops {
        let top_and_anträge = top_with_anträge(repo, top.id).await?.unwrap();

        tops_with_anträge.push(top_and_anträge);
    }

    Ok(tops_with_anträge)
}

pub async fn sitzung_with_tops(
    repo: &mut impl SitzungAntragService,
    sitzung_id: Uuid,
) -> Result<Option<SitzungWithTops>> {
    let Some(sitzung) = repo.sitzung_by_id(sitzung_id).await? else {
        return Ok(None);
    };

    let tops_with_anträge = top_with_anträge_by_sitzung(repo, sitzung_id).await?;

    Ok(Some(SitzungWithTops {
        sitzung,
        tops: tops_with_anträge,
    }))
}

pub async fn sitzungen_after_with_tops(
    repo: &mut impl SitzungAntragService,
    timestamp: DateTime<Utc>,
    limit: Option<i64>,
) -> Result<Option<SitzungenWithTops>> {
    let sitzungen = repo.sitzungen_after(timestamp, limit).await? else {
        return Ok(None);
    };

    let mut sitzungen_with_tops = vec![];

    for sitzung in &sitzungen {
        let tops = repo.tops_by_sitzung(sitzung.id).await?;
        let mut tops_with_anträge = vec![];
        for top in tops {
            let top_and_anträge = top_with_anträge(repo, top.id).await?.unwrap();

            tops_with_anträge.push(top_and_anträge);
        }
        sitzungen_with_tops.push(SitzungWithTops {
            sitzung: sitzung.clone(),
            tops: tops_with_anträge,
        });
    }

    Ok(Some(SitzungenWithTops {
        sitzungen: sitzungen_with_tops,
    }))
}

pub async fn abmeldungen_by_sitzung(
    repo: &mut impl SitzungPersonService,
    sitzung_id: Uuid,
) -> Result<Option<Vec<Abmeldung>>> {
    let Some(sitzung) = repo.sitzung_by_id(sitzung_id).await? else {
        return Ok(None);
    };

    let abmeldungen = repo.abmeldungen_at(sitzung.datetime.date_naive()).await?;

    Ok(Some(abmeldungen))
}
