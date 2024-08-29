import { invoke } from '@tauri-apps/api';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

const createFeatureStore = () => {
	const features = writable({
		devWay: false,
		restartEcsService: false,
		startEcsProxy: false,
		startRdsProxy: false,
		startLambdaProxy: false,
		getRdsSecret: false,
		proxyCustomHeaders: false,
		lambdaApps: false,
		prodActionsEnabled: false
	});

	async function refreshFeatures() {
		return Promise.all([
			invoke<boolean>('is_feature_enabled', { feature: 'dev-way' }),
			invoke<boolean>('is_feature_enabled', { feature: 'restart-ecs-service' }),
			invoke<boolean>('is_feature_enabled', { feature: 'start-ecs-proxy' }),
			invoke<boolean>('is_feature_enabled', { feature: 'start-rds-proxy' }),
			invoke<boolean>('is_feature_enabled', { feature: 'start-lambda-proxy' }),
			invoke<boolean>('is_feature_enabled', { feature: 'get-rds-secret' }),
			invoke<boolean>('is_feature_enabled', { feature: 'proxy-custom-headers' }),
			invoke<boolean>('is_feature_enabled', { feature: 'lambda-apps' }),
			invoke<boolean>('is_feature_enabled', { feature: 'prod-actions-enabled' })
		]).then(
			([
				devWay,
				restartEcsService,
				startEcsProxy,
				startRdsProxy,
				startLambdaProxy,
				getRdsSecret,
				proxyCustomHeaders,
				lambdaApps,
				prodActionsEnabled
			]) => {
				const newFs = {
					devWay,
					restartEcsService,
					startEcsProxy,
					startRdsProxy,
					startLambdaProxy,
					getRdsSecret,
					proxyCustomHeaders,
					lambdaApps,
					prodActionsEnabled
				};
				features.set(newFs);
			}
		);
	}
	refreshFeatures();
	listen('cache-refreshed', () => {
		refreshFeatures();
	});

	return { ...features, refreshFeatures };
};

export const featuresStore = createFeatureStore();
