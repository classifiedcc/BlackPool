use super::*;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BlockTemplate {
    pub bits: Nbits,
    pub previous_block_hash: BlockHash,
    pub current_time: Ntime,
    pub height: u64,
    pub version: Version,
    pub transactions: Vec<TemplateTransaction>,
    pub default_witness_commitment: ScriptBuf,
    pub coinbaseaux: BTreeMap<String, String>,
    pub coinbase_value: Amount,
    pub merkle_branches: Vec<MerkleNode>,
}

impl Default for BlockTemplate {
    fn default() -> Self {
        Self {
            bits: Nbits::from(CompactTarget::from_consensus(0)),
            previous_block_hash: BlockHash::all_zeros(),
            current_time: Ntime::from(0),
            height: 0,
            version: Version::from(block::Version::TWO),
            transactions: Vec::new(),
            default_witness_commitment: ScriptBuf::new(),
            coinbaseaux: BTreeMap::new(),
            coinbase_value: Amount::from_sat(COIN_VALUE),
            merkle_branches: Vec::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
pub(crate) struct GetBlockTemplate {
    #[serde(default)]
    pub(crate) bits: Option<String>,
    #[serde(default)]
    pub(crate) target: Option<String>,
    #[serde(rename = "previousblockhash")]
    pub(crate) previous_block_hash: BlockHash,
    #[serde(rename = "curtime", deserialize_with = "ntime_from_u64")]
    pub(crate) current_time: Ntime,
    pub(crate) height: u64,
    #[serde(deserialize_with = "version_from_i32")]
    pub(crate) version: Version,
    pub(crate) transactions: Vec<TemplateTransaction>,
    #[serde(with = "bitcoin::script::ScriptBuf", default)]
    pub(crate) default_witness_commitment: ScriptBuf,
    pub(crate) coinbaseaux: BTreeMap<String, String>,
    #[serde(
        rename = "coinbasevalue",
        with = "bitcoin::amount::serde::as_sat",
        default
    )]
    pub(crate) coinbase_value: Amount,
}

impl GetBlockTemplate {
    fn resolve_bits(&self) -> Result<Nbits, String> {
        if let Some(ref bits_hex) = self.bits {
            Nbits::from_str(bits_hex).map_err(|e| e.to_string())
        } else if let Some(ref target_hex) = self.target {
            let bytes = hex::decode(target_hex).map_err(|e| e.to_string())?;
            let arr: [u8; 32] = bytes
                .try_into()
                .map_err(|_| "target must be 32 bytes (64 hex chars)".to_string())?;
            let target = Target::from_be_bytes(arr);
            Ok(Nbits::from(target.to_compact_lossy()))
        } else {
            Err("getblocktemplate must include 'bits' or 'target'".to_string())
        }
    }
}

impl TryFrom<GetBlockTemplate> for BlockTemplate {
    type Error = String;

    fn try_from(raw: GetBlockTemplate) -> Result<Self, Self::Error> {
        let bits = raw.resolve_bits()?;
        let merkle_branches =
            stratum::merkle_branches(raw.transactions.iter().map(|tx| tx.txid).collect());

        Ok(Self {
            bits,
            previous_block_hash: raw.previous_block_hash,
            current_time: raw.current_time,
            height: raw.height,
            version: raw.version,
            transactions: raw.transactions,
            default_witness_commitment: raw.default_witness_commitment,
            coinbaseaux: raw.coinbaseaux,
            coinbase_value: raw.coinbase_value,
            merkle_branches,
        })
    }
}


#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct TemplateTransaction {
    pub txid: Txid,
    #[serde(rename = "data", deserialize_with = "tx_from_hex")]
    pub transaction: Transaction,
}

fn version_from_i32<'de, D>(d: D) -> Result<Version, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let x = i32::deserialize(d)?;
    Ok(Version::from(x))
}

fn tx_from_hex<'de, D>(d: D) -> Result<Transaction, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <&str>::deserialize(d)?;
    encode::deserialize_hex(s).map_err(serde::de::Error::custom)
}

fn ntime_from_u64<'de, D>(d: D) -> Result<Ntime, D::Error>
where
    D: Deserializer<'de>,
{
    let v = u64::deserialize(d)?;
    Ntime::try_from(v).map_err(de::Error::custom)
}
