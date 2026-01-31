import { invoke } from '@tauri-apps/api/core';

import type { LogFilter } from '$lib/types';
import { derived, writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';

const logFilters = writable<LogFilter[]>();
const loading = writable(false);
const refresh = () => {
	loading.set(true);
	console.log('refreshing');
	setTimeout(() => {
		invoke<LogFilter[]>('log_filters')
			.then(logFilters.set)
			.catch(() => {
				logFilters.set([]);
			})
			.finally(() => {
				loading.set(false);
			});
	}, 3000);
};
refresh();

listen('cache-refreshed', () => {
	refresh();
});

const save = (filters: LogFilter[]) => {
	loading.set(true);
	invoke<void>('save_log_filters', { filters })
		.then(() => {
			logFilters.set(filters);
		})
		.finally(() => {
			loading.set(false);
		});
};

export const logFiltersStore = derived([logFilters, loading], ([filters, isLoading]) => {
	return {
		filters,
		isLoading,
		refresh,
		save
	};
});
