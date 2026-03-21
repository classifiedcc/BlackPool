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

fn resolve_bits_from_obj(obj: &serde_json::Map<String, serde_json::Value>) -> Result<Nbits, String> {
    if let Some(bits_hex) = obj.get("bits").and_then(|v| v.as_str()) {
        return Nbits::from_str(bits_hex).map_err(|e| e.to_string());
    }
    if let Some(target_hex) = obj.get("target").and_then(|v| v.as_str()) {
        let bytes = hex::decode(target_hex).map_err(|e| e.to_string())?;
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "target must be 32 bytes (64 hex chars)".to_string())?;
        let target = Target::from_be_bytes(arr);
        return Ok(Nbits::from(target.to_compact_lossy()));
    }
    Err("getblocktemplate must include 'bits' or 'target'".to_string())
}

impl BlockTemplate {
    /// Parse from raw getblocktemplate JSON. Supports both "bits" (legacy) and "target" (Bitcoin Core 28+).
    pub(crate) fn from_json_value(mut v: serde_json::Value) -> Result<Self, String> {
        // call_raw returns the "result" object directly, but handle both wrapped and unwrapped
        if let Some(obj) = v.get("result").and_then(|r| r.as_object()) {
            v = serde_json::Value::Object(obj.clone());
        }
        let obj = v.as_object().ok_or("expected JSON object")?;

        let bits = resolve_bits_from_obj(obj)?;
        let previous_block_hash = obj
            .get("previousblockhash")
            .and_then(|v| v.as_str())
            .ok_or("missing previousblockhash")?
            .parse()
            .map_err(|e| format!("invalid previousblockhash: {e}"))?;
        let current_time = obj
            .get("curtime")
            .and_then(|v| v.as_u64())
            .ok_or("missing curtime")?;
        let height = obj
            .get("height")
            .and_then(|v| v.as_u64())
            .ok_or("missing height")?;
        let version = obj
            .get("version")
            .and_then(|v| v.as_i64())
            .ok_or("missing version")?;
        let transactions: Vec<TemplateTransaction> = obj
            .get("transactions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| {
                        let txid = t.get("txid")?.as_str()?.parse().ok()?;
                        let data = t.get("data")?.as_str()?;
                        let transaction =
                            consensus::encode::deserialize_hex::<Transaction>(data).ok()?;
                        Some(TemplateTransaction { txid, transaction })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let default_witness_commitment = obj
            .get("default_witness_commitment")
            .and_then(|v| v.as_str())
            .and_then(|s| hex::decode(s).ok())
            .map(ScriptBuf::from_bytes)
            .unwrap_or_default();
        let coinbaseaux = obj
            .get("coinbaseaux")
            .and_then(|v| v.as_object())
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        let coinbase_value = obj
            .get("coinbasevalue")
            .and_then(|v| v.as_i64())
            .map(|v| Amount::from_sat(v as u64))
            .unwrap_or(Amount::from_sat(COIN_VALUE));

        let merkle_branches =
            stratum::merkle_branches(transactions.iter().map(|tx| tx.txid).collect());

        Ok(Self {
            bits,
            previous_block_hash,
            current_time: current_time.try_into().map_err(|e| format!("curtime: {e}"))?,
            height,
            version: Version::from(version as i32),
            transactions,
            default_witness_commitment,
            coinbaseaux,
            coinbase_value,
            merkle_branches,
        })
    }
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
