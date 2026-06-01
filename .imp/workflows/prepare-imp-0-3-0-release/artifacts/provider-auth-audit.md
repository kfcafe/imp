# Provider/auth audit

## README provider claims
## Providers

Provider families:

- Anthropic
- OpenAI / ChatGPT
- Google
- OpenAI-compatible APIs
- Moonshot / Kimi
- Z.AI / GLM
- DeepSeek
- Groq
- Cerebras
- xAI
- Mistral
- Together
- OpenRouter
- Fireworks

API-key configuration:

```bash
export ANTHROPIC_API_KEY=...
export OPENAI_API_KEY=...
export GOOGLE_API_KEY=...
export OPENROUTER_API_KEY=...
```

Credential commands:

```bash
imp login
imp login openai
imp login kimi
imp secrets list
imp secrets doctor
```

## Tools

## Provider registry/auth references
crates/imp-cli/src/acp/mod.rs:489:    use imp_llm::auth::{ApiKey, AuthStore};
crates/imp-cli/src/acp/mod.rs:509:            _api_key: &str,
crates/imp-cli/src/acp/mod.rs:531:        async fn resolve_auth(&self, _auth: &AuthStore) -> imp_llm::Result<ApiKey> {
crates/imp-cli/src/acp/mod.rs:571:        agent.api_key = "test-key".to_string();
crates/imp-llm/src/model.rs:11:    /// Native Anthropic Messages API.
crates/imp-llm/src/model.rs:12:    Anthropic,
crates/imp-llm/src/model.rs:13:    /// Native OpenAI Responses API.
crates/imp-llm/src/model.rs:15:    /// ChatGPT/Codex-backed OpenAI Responses API.
crates/imp-llm/src/model.rs:19:    /// OpenAI-compatible Chat Completions API (DeepSeek, Groq, etc.).
crates/imp-llm/src/model.rs:25:pub struct ProviderMeta {
crates/imp-llm/src/model.rs:26:    /// Provider identifier (e.g. "anthropic", "deepseek").
crates/imp-llm/src/model.rs:28:    /// Human-readable name (e.g. "Anthropic", "DeepSeek").
crates/imp-llm/src/model.rs:43:    providers: Vec<ProviderMeta>,
crates/imp-llm/src/model.rs:61:    /// Find a provider by its id (e.g. "anthropic", "deepseek").
crates/imp-llm/src/model.rs:62:    pub fn find(&self, id: &str) -> Option<&ProviderMeta> {
crates/imp-llm/src/model.rs:67:    pub fn list(&self) -> &[ProviderMeta] {
crates/imp-llm/src/model.rs:79:pub fn builtin_providers() -> Vec<ProviderMeta> {
crates/imp-llm/src/model.rs:81:        ProviderMeta {
crates/imp-llm/src/model.rs:82:            id: "anthropic",
crates/imp-llm/src/model.rs:83:            name: "Anthropic",
crates/imp-llm/src/model.rs:84:            env_vars: &["ANTHROPIC_API_KEY"],
crates/imp-llm/src/model.rs:86:            docs_url: "console.anthropic.com/settings/keys",
crates/imp-llm/src/model.rs:87:            api_style: ApiStyle::Anthropic,
crates/imp-llm/src/model.rs:89:        ProviderMeta {
crates/imp-llm/src/model.rs:90:            id: "openai",
crates/imp-llm/src/model.rs:91:            name: "OpenAI",
crates/imp-llm/src/model.rs:92:            env_vars: &["OPENAI_API_KEY"],
crates/imp-llm/src/model.rs:94:            docs_url: "platform.openai.com/api-keys",
crates/imp-llm/src/model.rs:97:        ProviderMeta {
crates/imp-llm/src/model.rs:98:            id: "openai-codex",
crates/imp-llm/src/model.rs:105:        ProviderMeta {
crates/imp-llm/src/model.rs:108:            env_vars: &["GOOGLE_API_KEY"],
crates/imp-llm/src/model.rs:113:        ProviderMeta {
crates/imp-llm/src/model.rs:116:            env_vars: &["DEEPSEEK_API_KEY"],
crates/imp-llm/src/model.rs:118:            docs_url: "platform.deepseek.com/api_keys",
crates/imp-llm/src/model.rs:121:        ProviderMeta {
crates/imp-llm/src/model.rs:124:            env_vars: &["MOONSHOT_API_KEY", "KIMI_API_KEY"],
crates/imp-llm/src/model.rs:129:        ProviderMeta {
crates/imp-llm/src/model.rs:132:            env_vars: &["KIMICODE_API_KEY"],
crates/imp-llm/src/model.rs:137:        ProviderMeta {
crates/imp-llm/src/model.rs:140:            env_vars: &["ZAI_API_KEY"],
crates/imp-llm/src/model.rs:145:        ProviderMeta {
crates/imp-llm/src/model.rs:148:            env_vars: &["OPENROUTER_API_KEY"],
crates/imp-llm/src/model.rs:153:        ProviderMeta {
crates/imp-llm/src/model.rs:156:            env_vars: &["GROQ_API_KEY"],
crates/imp-llm/src/model.rs:157:            api_base_url: Some("https://api.groq.com/openai"),
crates/imp-llm/src/model.rs:169:    /// Provider that serves this model (e.g. "anthropic").
crates/imp-llm/src/model.rs:244:    /// Anthropic, OpenAI, and Google.
crates/imp-llm/src/model.rs:334:        // -- Anthropic --
crates/imp-llm/src/model.rs:338:            provider: "anthropic".into(),
crates/imp-llm/src/model.rs:357:            provider: "anthropic".into(),
crates/imp-llm/src/model.rs:376:            provider: "anthropic".into(),
crates/imp-llm/src/model.rs:659:    let openai_insert_at = models
crates/imp-llm/src/model.rs:661:        .take_while(|model| model.provider == "anthropic")
crates/imp-llm/src/model.rs:663:    models.splice(openai_insert_at..openai_insert_at, builtin_openai_models());
crates/imp-llm/src/model.rs:667:pub fn builtin_openai_models() -> Vec<ModelMeta> {
crates/imp-llm/src/model.rs:671:            provider: "openai".into(),
crates/imp-llm/src/model.rs:689:            provider: "openai".into(),
crates/imp-llm/src/model.rs:707:            provider: "openai".into(),
crates/imp-llm/src/model.rs:725:            provider: "openai".into(),
crates/imp-llm/src/model.rs:743:            provider: "openai".into(),
crates/imp-llm/src/model.rs:761:            provider: "openai".into(),
crates/imp-llm/src/model.rs:775:pub fn builtin_openai_codex_models() -> Vec<ModelMeta> {
crates/imp-llm/src/model.rs:776:    let mut models: Vec<ModelMeta> = builtin_openai_models()
crates/imp-llm/src/model.rs:779:            model.provider = "openai-codex".into();
crates/imp-llm/src/model.rs:786:        provider: "openai-codex".into(),
crates/imp-llm/src/model.rs:811:        return Some("openai");
crates/imp-llm/src/model.rs:815:        return Some("anthropic");
crates/imp-llm/src/model.rs:835:        "openai" => synthesize_openai_model_meta(model_id),
crates/imp-llm/src/model.rs:836:        "openai-codex" => {
crates/imp-llm/src/model.rs:837:            let mut meta = synthesize_openai_model_meta(model_id);
crates/imp-llm/src/model.rs:838:            meta.provider = "openai-codex".into();
crates/imp-llm/src/model.rs:841:        "anthropic" => ModelMeta {
crates/imp-llm/src/model.rs:915:fn synthesize_openai_model_meta(model_id: &str) -> ModelMeta {
crates/imp-llm/src/model.rs:919:            provider: "openai".into(),
crates/imp-llm/src/model.rs:937:            provider: "openai".into(),
crates/imp-llm/src/model.rs:955:            provider: "openai".into(),
crates/imp-llm/src/model.rs:973:            provider: "openai".into(),
crates/imp-llm/src/model.rs:986:            provider: "openai".into(),
crates/imp-llm/src/model.rs:999:            provider: "openai".into(),
crates/imp-llm/src/model.rs:1012:            provider: "openai".into(),
crates/imp-llm/src/model.rs:1025:            provider: "openai".into(),
crates/imp-llm/src/model.rs:1038:            provider: "openai".into(),
crates/imp-llm/src/model.rs:1054:        // Anthropic — sonnet
crates/imp-llm/src/model.rs:1058:        // Anthropic — haiku
crates/imp-llm/src/model.rs:1062:        // Anthropic — opus
crates/imp-llm/src/model.rs:1066:        // OpenAI
crates/imp-llm/src/model.rs:1136:        assert_eq!(model.provider, "anthropic");
crates/imp-llm/src/model.rs:1173:        assert_eq!(model.provider, "openai");
crates/imp-llm/src/model.rs:1202:        assert_eq!(model.provider, "openai");
crates/imp-llm/src/model.rs:1206:    fn resolve_meta_synthesizes_legacy_openai_model() {
crates/imp-llm/src/model.rs:1210:            .expect("legacy openai model should synthesize");
crates/imp-llm/src/model.rs:1212:        assert_eq!(model.provider, "openai");
crates/imp-llm/src/model.rs:1279:        assert_eq!(provider.env_vars, &["ZAI_API_KEY"]);
crates/imp-llm/src/model.rs:1290:        assert_eq!(provider.env_vars, &["MOONSHOT_API_KEY", "KIMI_API_KEY"]);
crates/imp-llm/src/model.rs:1311:        let anthropic = reg.list_by_provider("anthropic");
crates/imp-llm/src/model.rs:1312:        assert_eq!(anthropic.len(), 3);
crates/imp-llm/src/model.rs:1313:        assert!(anthropic.iter().all(|m| m.provider == "anthropic"));
crates/imp-llm/src/model.rs:1315:        let openai = reg.list_by_provider("openai");
crates/imp-llm/src/model.rs:1316:        assert_eq!(openai.len(), 6);
crates/imp-llm/src/model.rs:1326:    fn builtin_openai_codex_models_retag_openai_models() {
crates/imp-llm/src/model.rs:1327:        let models = builtin_openai_codex_models();
crates/imp-llm/src/model.rs:1329:        assert!(models.iter().all(|model| model.provider == "openai-codex"));
crates/imp-llm/src/model.rs:1333:            .expect("OpenAI Codex model list should include GPT-5.5");
crates/imp-core/src/context.rs:181:            _api_key: &str,
crates/imp-core/src/context.rs:188:            _auth: &imp_llm::auth::AuthStore,
crates/imp-llm/src/auth.rs:223:            "openai" | "openai-codex" => {
crates/imp-llm/src/auth.rs:224:                let mut message = String::from("Logged in to OpenAI / ChatGPT");
crates/imp-llm/src/auth.rs:234:            "anthropic" => {
crates/imp-llm/src/auth.rs:236:                    format!("Logged in to Anthropic with {plan} subscription credentials.")
crates/imp-llm/src/auth.rs:238:                    "Logged in to Anthropic with OAuth subscription credentials.".into()
crates/imp-llm/src/auth.rs:263:pub struct AuthStore {
crates/imp-llm/src/auth.rs:270:impl AuthStore {
crates/imp-llm/src/auth.rs:337:        self.resolve_secret_field(provider, "api_key")
crates/imp-llm/src/auth.rs:341:    pub fn resolve_api_key_only(&self, provider: &str) -> Result<ApiKey> {
crates/imp-llm/src/auth.rs:342:        self.resolve_secret_field(provider, "api_key")
crates/imp-llm/src/auth.rs:347:        if field == "api_key" {
crates/imp-llm/src/auth.rs:355:                StoredCredential::ApiKey { key } if field == "api_key" => return Ok(key.clone()),
crates/imp-llm/src/auth.rs:442:                "api_key".to_string(),
crates/imp-llm/src/auth.rs:476:                Ok(HashMap::from([("api_key".to_string(), key.clone())]))
crates/imp-llm/src/auth.rs:483:                if let Some(api_key) = resolve_env_secret(provider, "api_key") {
crates/imp-llm/src/auth.rs:484:                    Ok(HashMap::from([("api_key".to_string(), api_key)]))
crates/imp-llm/src/auth.rs:486:                    Err(missing_secret_error(provider, "api_key"))
crates/imp-llm/src/auth.rs:492:    /// Resolve a ChatGPT/OpenAI OAuth token, preferring `openai-codex` when present.
crates/imp-llm/src/auth.rs:494:        for provider in ["openai-codex", "openai"] {
crates/imp-llm/src/auth.rs:514:            "No ChatGPT OAuth credential found. Run `imp login openai` or configure an OpenAI API key."
crates/imp-llm/src/auth.rs:537:                    "anthropic" => {
crates/imp-llm/src/auth.rs:538:                        crate::oauth::anthropic::AnthropicOAuth::new()
crates/imp-llm/src/auth.rs:685:            | "api_key"
crates/imp-llm/src/auth.rs:707:                "api_key",
crates/imp-llm/src/auth.rs:728:    if field == "api_key" {
crates/imp-llm/src/auth.rs:765:        "anthropic" => Some(OAuthDisplayInfo {
crates/imp-llm/src/auth.rs:770:        "openai" | "openai-codex" => decode_openai_oauth_display_info(&credential.access_token),
crates/imp-llm/src/auth.rs:780:fn decode_openai_oauth_display_info(access_token: &str) -> Option<OAuthDisplayInfo> {
crates/imp-llm/src/auth.rs:784:    let auth = claims.get("https://api.openai.com/auth")?;
crates/imp-llm/src/auth.rs:855:    fn test_store(path: std::path::PathBuf) -> AuthStore {
crates/imp-llm/src/auth.rs:856:        AuthStore::new_with_backend(path, Arc::new(MockSecretBackend::default()))
crates/imp-llm/src/auth.rs:862:    ) -> AuthStore {
crates/imp-llm/src/auth.rs:863:        AuthStore::new_with_backend(path, backend)
crates/imp-llm/src/auth.rs:869:    ) -> AuthStore {
crates/imp-llm/src/auth.rs:870:        AuthStore::load_with_backend(path, backend).unwrap()
crates/imp-llm/src/auth.rs:873:    fn jwt_with_openai_auth(plan: &str, account_id: &str) -> String {
crates/imp-llm/src/auth.rs:877:                "https://api.openai.com/auth": {
crates/imp-llm/src/auth.rs:937:            .store("anthropic", StoredCredential::OAuth(cred))
crates/imp-llm/src/auth.rs:940:        let key = store.resolve("anthropic").unwrap();
crates/imp-llm/src/auth.rs:950:        fields.insert("api_key".to_string(), "test-api".to_string());
crates/imp-llm/src/auth.rs:959:                .resolve_secret_field("test-service", "api_key")
crates/imp-llm/src/auth.rs:979:            HashMap::from([("api_key".to_string(), "test-api".to_string())]),
crates/imp-llm/src/auth.rs:997:                    ("api_key".to_string(), "test-api".to_string()),
crates/imp-llm/src/auth.rs:1006:            resolved.get("api_key").map(String::as_str),
crates/imp-llm/src/auth.rs:1025:                    ("api_key".to_string(), "test-api".to_string()),
crates/imp-llm/src/auth.rs:1033:            .resolve_secret_field("test-service", "api_key")
crates/imp-llm/src/auth.rs:1035:        assert!(backend.get("test-service", "api_key").unwrap().is_none());
crates/imp-llm/src/auth.rs:1050:            .store("anthropic", StoredCredential::OAuth(fresh))
crates/imp-llm/src/auth.rs:1052:        assert!(!store.is_oauth_expired("anthropic"));
crates/imp-llm/src/auth.rs:1060:            .store("anthropic", StoredCredential::OAuth(expired))
crates/imp-llm/src/auth.rs:1062:        assert!(store.is_oauth_expired("anthropic"));
crates/imp-llm/src/auth.rs:1077:            .store("anthropic", StoredCredential::OAuth(expired))
crates/imp-llm/src/auth.rs:1081:            .resolve_or_refresh("anthropic", |refresh_tok| {
crates/imp-llm/src/auth.rs:1096:        let resolved = store.resolve("anthropic").unwrap();
crates/imp-llm/src/auth.rs:1112:            .store("anthropic", StoredCredential::OAuth(fresh))
crates/imp-llm/src/auth.rs:1116:            .resolve_or_refresh("anthropic", |_| async {
crates/imp-llm/src/auth.rs:1132:        let err = match AuthStore::load_with_backend(&path, backend) {
crates/imp-llm/src/auth.rs:1149:                "openai",
crates/imp-llm/src/auth.rs:1159:        assert_eq!(loaded.resolve("openai").unwrap(), "sk-atomic");
crates/imp-llm/src/auth.rs:1176:                .store("anthropic", StoredCredential::OAuth(cred))
crates/imp-llm/src/auth.rs:1181:        let key = store.resolve("anthropic").unwrap();
crates/imp-llm/src/auth.rs:1197:            .store("anthropic", StoredCredential::OAuth(cred))
crates/imp-llm/src/auth.rs:1199:        assert!(store.resolve("anthropic").is_ok());
crates/imp-llm/src/auth.rs:1201:        store.remove("anthropic").unwrap();
crates/imp-llm/src/auth.rs:1202:        std::env::remove_var("ANTHROPIC_API_KEY");
crates/imp-llm/src/auth.rs:1203:        assert!(store.resolve("anthropic").is_err());
crates/imp-llm/src/auth.rs:1214:                "anthropic",
crates/imp-llm/src/auth.rs:1221:        store.set_runtime_key("anthropic", "runtime-key".into());
crates/imp-llm/src/auth.rs:1222:        let key = store.resolve("anthropic").unwrap();
crates/imp-llm/src/auth.rs:1232:        store.set_runtime_key("openai", "runtime-key".into());
crates/imp-llm/src/auth.rs:1233:        assert_eq!(store.resolve("openai").unwrap(), "runtime-key");
crates/imp-llm/src/auth.rs:1235:        store.set_runtime_key("openai", "   ".into());
crates/imp-llm/src/auth.rs:1236:        assert!(store.resolve("openai").is_err());
crates/imp-llm/src/auth.rs:1240:    fn test_resolve_stored_api_key() {
crates/imp-llm/src/auth.rs:1247:                "openai",
crates/imp-llm/src/auth.rs:1254:        let key = store.resolve("openai").unwrap();
crates/imp-llm/src/auth.rs:1264:        std::env::remove_var("KIMI_API_KEY");
crates/imp-llm/src/auth.rs:1265:        std::env::set_var("MOONSHOT_API_KEY", "moonshot-env-key");
crates/imp-llm/src/auth.rs:1268:        std::env::remove_var("MOONSHOT_API_KEY");
crates/imp-llm/src/auth.rs:1270:        std::env::set_var("KIMI_API_KEY", "kimi-env-key");
crates/imp-llm/src/auth.rs:1273:        std::env::remove_var("KIMI_API_KEY");
crates/imp-llm/src/auth.rs:1277:    fn test_resolve_api_key_only_ignores_oauth_credentials() {
crates/imp-llm/src/auth.rs:1284:                "openai",
crates/imp-llm/src/auth.rs:1293:        assert!(store.resolve_api_key_only("openai").is_err());
crates/imp-llm/src/auth.rs:1297:    async fn test_resolve_chatgpt_oauth_prefers_openai_codex() {
crates/imp-llm/src/auth.rs:1304:                "openai",
crates/imp-llm/src/auth.rs:1306:                    access_token: "openai-oauth".into(),
crates/imp-llm/src/auth.rs:1307:                    refresh_token: "openai-refresh".into(),
crates/imp-llm/src/auth.rs:1314:                "openai-codex",
crates/imp-llm/src/auth.rs:1328:    async fn test_resolve_chatgpt_oauth_falls_back_to_openai() {
crates/imp-llm/src/auth.rs:1335:                "openai",
crates/imp-llm/src/auth.rs:1337:                    access_token: "openai-oauth".into(),
crates/imp-llm/src/auth.rs:1338:                    refresh_token: "openai-refresh".into(),
crates/imp-llm/src/auth.rs:1345:        assert_eq!(key, "openai-oauth");
crates/imp-llm/src/auth.rs:1349:    fn test_oauth_display_info_for_openai_credential() {
crates/imp-llm/src/auth.rs:1351:            access_token: jwt_with_openai_auth("pro", "acct-12345678"),
crates/imp-llm/src/auth.rs:1356:        let info = oauth_display_info_for_credential("openai", &credential).unwrap();
crates/imp-llm/src/auth.rs:1363:    fn test_oauth_display_info_for_anthropic_credential() {
crates/imp-llm/src/auth.rs:1370:        let info = oauth_display_info_for_credential("anthropic", &credential).unwrap();
crates/imp-llm/src/auth.rs:1374:            info.login_message("anthropic"),
crates/imp-llm/src/auth.rs:1375:            "Logged in to Anthropic with Claude Max/Pro subscription credentials."
crates/imp-llm/src/auth.rs:1396:        std::env::remove_var("GOOGLE_API_KEY");
crates/imp-llm/src/auth.rs:1430:                HashMap::from([("api_key".to_string(), "render-secret".to_string())]),
crates/imp-llm/src/auth.rs:1436:            fields.get("api_key").map(String::as_str),
crates/imp-llm/src/auth.rs:1449:                fields: vec!["api_key".into()],
crates/imp-llm/src/auth.rs:1457:            vec![("api_key".to_string(), SecretFieldStatus::Missing)]
crates/imp-llm/src/auth.rs:1469:        std::env::remove_var("MOONSHOT_API_KEY");
crates/imp-llm/src/auth.rs:1470:        std::env::set_var("KIMI_API_KEY", "kimi-env-key");
crates/imp-llm/src/auth.rs:1472:        std::env::remove_var("KIMI_API_KEY");
crates/imp-cli/src/usage_report.rs:926:            provider: Some("anthropic".into()),
crates/imp-cli/src/usage_report.rs:938:            provider: Some("anthropic".into()),
crates/imp-cli/src/usage_report.rs:951:            provider: Some("openai".into()),
crates/imp-cli/src/usage_report.rs:963:                "anthropic",
crates/imp-cli/src/usage_report.rs:976:                "anthropic",
crates/imp-cli/src/usage_report.rs:996:        assert_eq!(models[0].group, "anthropic/claude");
crates/imp-cli/src/usage_report.rs:1009:                "anthropic",
crates/imp-cli/src/usage_report.rs:1022:                "openai",
crates/imp-llm/src/provider.rs:8:use crate::auth::{ApiKey, AuthStore};
crates/imp-llm/src/provider.rs:16:/// Each provider (Anthropic, OpenAI, Google, etc.) implements this trait
crates/imp-llm/src/provider.rs:26:        api_key: &str,
crates/imp-llm/src/provider.rs:30:    async fn resolve_auth(&self, auth: &AuthStore) -> Result<ApiKey>;
crates/imp-llm/src/provider.rs:32:    /// Provider identifier (e.g., "anthropic", "openai", "google").
crates/imp-llm/src/provider.rs:127:    /// Effort level for the model (Anthropic-specific).
crates/imp-llm/src/provider.rs:147:/// Only supported by Anthropic models with the effort beta.
crates/imp-llm/src/lib.rs:23:    ApiStyle, Capabilities, Model, ModelMeta, ModelPricing, ModelRegistry, ProviderMeta,
crates/imp-core/src/builder.rs:45:    api_key: String,
crates/imp-core/src/builder.rs:76:    pub fn new(config: Config, cwd: PathBuf, model: Model, api_key: String) -> Self {
crates/imp-core/src/builder.rs:81:            api_key,
crates/imp-core/src/builder.rs:301:        agent.api_key = self.api_key;
crates/imp-core/src/builder.rs:473:        auth::{ApiKey, AuthStore},
crates/imp-core/src/builder.rs:488:            _api_key: &str,
crates/imp-core/src/builder.rs:493:        async fn resolve_auth(&self, _auth: &AuthStore) -> imp_llm::Result<ApiKey> {
crates/imp-core/src/builder.rs:591:    fn builder_api_key_wired() {
crates/imp-core/src/builder.rs:601:        assert_eq!(agent.api_key, "my-api-key");
