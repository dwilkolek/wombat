import { derived, get, writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AwsEnv, RdsInstance, ServiceDetails } from '$lib/types';
import { ENVIRONMENTS } from './env-store';

import { format } from 'date-fns';

type ServiceDetailsPayload = {
	app: string;
	dbs: RdsInstance[];
	services: ServiceDetails[];
	timestamp: number;
};

const createServiceDetailsStore = () => {
	const innerStore = writable<ServiceDetailsPayload[]>([]);
	const refreshOne = (app: string) => {
		innerStore.update((old) => {
			return old.filter((o) => o.app !== app);
		});
	}
	return { ...innerStore, refreshOne };
};

listen<ServiceDetailsPayload>('new-service-details', (data) => {
	setTimeout(() => {
		delete to[data.payload.app];
	}, 200);
	allServiceDetailsStore.update((old) => {
		return [...old.filter((o) => o.app !== data.payload['app']), { ...data.payload }];
	});
});
listen('cache-refreshed', () => {
	allServiceDetailsStore.set([]);
});
listen('logged-out', () => {
	allServiceDetailsStore.set([]);
});
export const allServiceDetailsStore = createServiceDetailsStore();
let refreshTimeout: number | undefined;
const refresh = () => {
	console.log('refresh');
	clearTimeout(refreshTimeout);
	refreshTimeout = setTimeout(
		() => {
			const apps = get(allServiceDetailsStore);
			apps.forEach((app) => {
				invoke('service_details', { app: app.app });
			});
			refresh();
		},
		5 * 60 * 1000
	);
};
refresh();

const to: { [key: string]: Promise<void> } = {};
export const serviceDetailStore = (app: string) =>
	derived([allServiceDetailsStore], (stores) => {
		const apps = stores[0];
		const details = apps.find((a) => a.app === app);
		if (!details) {
			if (!to[app]) {
				to[app] = invoke('service_details', { app });
			}
			return undefined;
		}
		if (details) {
			const result = {
				app,
				envs: new Map<AwsEnv, { services: ServiceDetails[]; dbs: RdsInstance[] }>(),
				timestamp: format(details.timestamp, 'yyyy-MM-dd HH:mm:ss')
			};
			for (const env of ENVIRONMENTS) {
				const services = details.services.filter((s) => s.env === env);
				const dbs = details.dbs.filter((s) => s.env === env);
				if (services.length || dbs.length) {
					result.envs.set(env, {
						services,
						dbs
					});
				}
			}

			return result;
		}
	});
