import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

const createFeatureStore = () => {
	const features = writable({
		ecsTab: false,
		rdsTab: false,
		devWay: false,
		restartEcsService: false
	});

	Promise.all([
		invoke<boolean>('is_feature_enabled', { feature: 'ecs-tab' }),
		invoke<boolean>('is_feature_enabled', { feature: 'rds-tab' }),
		invoke<boolean>('is_feature_enabled', { feature: 'dev-way' }),
		invoke<boolean>('is_feature_enabled', { feature: 'restart-ecs-service' })
	]).then(([ecsTab, rdsTab, devWay, restartEcsService]) => {
		features.set({
			ecsTab,
			rdsTab,
			devWay,
			restartEcsService
		});
	});
	listen('cache-refreshed', () => {
		Promise.all([
			invoke<boolean>('is_feature_enabled', { feature: 'ecs-tab' }),
			invoke<boolean>('is_feature_enabled', { feature: 'rds-tab' }),
			invoke<boolean>('is_feature_enabled', { feature: 'dev-way' }),
			invoke<boolean>('is_feature_enabled', { feature: 'restart-ecs-service' })
		]).then(([ecsTab, rdsTab, devWay, restartEcsService]) => {
			features.set({
				ecsTab,
				rdsTab,
				devWay,
				restartEcsService
			});
		});
	});
	return features;
};

export const featuresStore = createFeatureStore();
