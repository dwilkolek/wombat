import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

const createFeatureStore = () => {
	const features = writable({
		ecsTab: false,
		rdsTab: false,
		allowAllProxies: false,
		allowAllSecrets: false
	});

	Promise.all([
		invoke<boolean>('is_feature_enabled', { feature: 'ecs-tab' }),
		invoke<boolean>('is_feature_enabled', { feature: 'rds-tab' }),
		invoke<boolean>('is_feature_enabled', { feature: 'allow-all-proxies' }),
		invoke<boolean>('is_feature_enabled', { feature: 'allow-all-secrets' })
	]).then(([ecsTab, rdsTab, allowAllProxies, allowAllSecrets ]) => {
		features.set({
			ecsTab,
			rdsTab,
			allowAllProxies,
			allowAllSecrets,
		});
	});
	listen('cache-refreshed', () => {
		Promise.all([
            invoke<boolean>('is_feature_enabled', { feature: 'ecs-tab' }),
            invoke<boolean>('is_feature_enabled', { feature: 'rds-tab' }),
            invoke<boolean>('is_feature_enabled', { feature: 'allow-all-proxies' }),
            invoke<boolean>('is_feature_enabled', { feature: 'allow-all-secrets' })
        ]).then(([ecsTab, rdsTab, allowAllProxies, allowAllSecrets ]) => {
            features.set({
                ecsTab,
                rdsTab,
                allowAllProxies,
                allowAllSecrets,
            });
        });
	});
	return features;
};

export const featuresStore = createFeatureStore();
