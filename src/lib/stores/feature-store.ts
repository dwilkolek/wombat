import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

const createFeatureStore = () => {
	const features = writable({
		ecsTab: false,
		rdsTab: false,
		devWay: false,
	});

	Promise.all([
		invoke<boolean>('is_feature_enabled', { feature: 'ecs-tab' }),
		invoke<boolean>('is_feature_enabled', { feature: 'rds-tab' }),
		invoke<boolean>('is_feature_enabled', { feature: 'dev-way' }),
	]).then(([ecsTab, rdsTab, devWay ]) => {
		features.set({
			ecsTab,
			rdsTab,
			devWay,
		});
	});
	listen('cache-refreshed', () => {
		Promise.all([
            invoke<boolean>('is_feature_enabled', { feature: 'ecs-tab' }),
            invoke<boolean>('is_feature_enabled', { feature: 'rds-tab' }),
            invoke<boolean>('is_feature_enabled', { feature: 'dev-way' }),
        ]).then(([ecsTab, rdsTab, devWay ]) => {
            features.set({
                ecsTab,
                rdsTab,
                devWay,
            });
        });
	});
	return features;
};

export const featuresStore = createFeatureStore();
