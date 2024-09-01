alter type sitzungtype rename to sitzungkind;

alter table sitzungen rename column sitzung_type to kind;
