use snaplingo_ui::{services::*, Snapshot, Theme};

pub fn fixture(route: ServicesRoute, theme: Theme, dark: bool, scenario: &str) -> ServicesSnapshot {
    let provider =
        |id: &str, name: &str, configured, active, local, requires_api_key| ServiceProvider {
            id: id.into(),
            name: name.into(),
            configured,
            active,
            builtin: true,
            local,
            requires_api_key,
            ..Default::default()
        };
    let mut ocr = vec![
        provider("system-ocr", "系统 OCR", true, true, true, false),
        provider("paddle-ocr", "PaddleOCR", true, false, true, false),
        provider("baidu-ocr", "百度 OCR", false, false, false, true),
    ];
    ocr[1].acceleration = Some("CPU 模式".into());
    let translation = vec![
        provider("google-translate", "Google 翻译", true, true, false, false),
        ServiceProvider {
            id: "custom-reference".into(),
            name: "我的 GPT".into(),
            configured: true,
            active: true,
            builtin: false,
            requires_api_key: true,
            protocol: Some(ServiceProtocol::OpenAiResponses),
            endpoint: Some("https://api.example.com".into()),
            model: Some("gpt-5-mini".into()),
            ..Default::default()
        },
        provider("deeplx", "DeepLX", true, false, false, true),
        provider("baidu-translate", "百度翻译", false, false, false, true),
    ];
    let form = match scenario {
        "ocr-credentials" | "translation-credentials" => {
            let ocr = route == ServicesRoute::Ocr;
            Some(ServiceForm::Credentials {
                provider_id: if ocr { "baidu-ocr" } else { "baidu-translate" }.into(),
                name: if ocr { "百度 OCR" } else { "百度翻译" }.into(),
                fields: vec![
                    ServiceCredentialField {
                        name: if ocr { "api_key" } else { "app_id" }.into(),
                        label: if ocr { "API Key" } else { "APP ID" }.into(),
                        secret: false,
                    },
                    ServiceCredentialField {
                        name: "secret_key".into(),
                        label: if ocr { "Secret Key" } else { "密钥" }.into(),
                        secret: true,
                    },
                ],
            })
        }
        "deeplx" | "deepl" => Some(ServiceForm::Credentials {
            provider_id: "deeplx".into(),
            name: "DeepLX".into(),
            fields: vec![],
        }),
        "custom" | "custom-prompt" => {
            let prompt = ServicePrompt {id:"legal".into(),name:"法律翻译".into(),description:"合同、条款、法律文本".into(),system_prompt:"你是一位专业法律译者。\n将 {source_lang} 翻译为 {target_lang}。\n保留条款结构与专有名词。".into(),builtin:false,deletable:true};
            Some(ServiceForm::Custom {
                draft: ServiceCustomDraft {
                    provider_id: Some("custom-reference".into()),
                    name: "我的 GPT".into(),
                    protocol: ServiceProtocol::OpenAiResponses,
                    endpoint: "https://api.example.com".into(),
                    model: "gpt-5-mini".into(),
                    reasoning_level: "medium".into(),
                    prompt_strategy_id: "legal".into(),
                    ..Default::default()
                },
                prompts: ServicePromptWorkspace {
                    draft: ServicePromptDraft {
                        name: prompt.name.clone(),
                        description: prompt.description.clone(),
                        system_prompt: prompt.system_prompt.clone(),
                    },
                    strategies: vec![prompt],
                    error: None,
                },
            })
        }
        _ => None,
    };
    ServicesSnapshot {
        shell: Snapshot {
            theme,
            dark,
            ..Default::default()
        },
        route,
        providers: if route == ServicesRoute::Ocr {
            ocr
        } else {
            translation
        },
        form,
        form_revision: 1,
        voices: vec![
            ServiceVoice {
                id: "com.fixture.voice.zh".into(),
                name: "婷婷".into(),
                locale: "zh-CN".into(),
            },
            ServiceVoice {
                id: "com.fixture.voice.en".into(),
                name: "Samantha".into(),
                locale: "en-US".into(),
            },
        ],
        voice_id: "com.fixture.voice.zh".into(),
        rate: 180,
        ..Default::default()
    }
}
