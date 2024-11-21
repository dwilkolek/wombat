import { derived, get, writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AwsEnv, RdsInstance, ServiceDetails, ServiceDetailsMissing } from '$lib/types';
import { ENVIRONMENTS } from './env-store';

type ServiceDetailsPayload = {
	app: string;
	dbs: RdsInstance[];
	services: { Ok?: ServiceDetails; Err?: ServiceDetailsMissing }[];
	timestamp: number;
};

const createServiceDetailsStore = () => {
	const innerStore = writable<ServiceDetailsPayload[]>([]);
	const refreshOne = (app: string) => {
		innerStore.update((old) => {
			return old.filter((o) => o.app !== app);
		});
	};
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
	to = {};
	allServiceDetailsStore.set([]);
});
listen('logged-out', () => {
	to = {};
	allServiceDetailsStore.set([]);
});

export const triggerSilentServiceDetailsRefresh = (app: string) => {
	to[app] = invoke('service_details', { app });
};

export const allServiceDetailsStore = createServiceDetailsStore();
let refreshTimeout: number | undefined;
const refresh = () => {
	console.log('refresh');
	clearTimeout(refreshTimeout);
	refreshTimeout = setTimeout(
		() => {
			const apps = get(allServiceDetailsStore);
			apps.forEach((app) => {
				if (!to[app.app]) {
					to[app.app] = invoke('service_details', { app: app.app });
				}
			});
			refresh();
		},
		15 * 60 * 1000
	);
};
refresh();

let to: { [key: string]: Promise<void> } = {};
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
				envs: new Map<
					AwsEnv,
					{ services: (ServiceDetails & ServiceDetailsMissing)[]; dbs: RdsInstance[] }
				>(),
				timestamp: details.timestamp
			};
			for (const env of ENVIRONMENTS) {
				const services = details.services
					.map((s) => s.Ok ?? s.Err)
					.filter((s) => s!.env === env) as (ServiceDetails & ServiceDetailsMissing)[];
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
