import type { AwsEnv, Timerange, TimeUnit } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { format } from 'date-fns';
import { writable, get } from 'svelte/store';
type LogEntry = {
	log_stream_name: string;
	timestamp: number;
	ingestion_time: number;
	message: string;
};
type LogStyle = {
	bg: string;
	active: string;
	hover: string;
};

type LogLevel = 'INFO' | 'WARN' | 'ERROR' | 'TRACE' | 'DEBUG' | 'UNKNOWN';
export type LogData = { [key: string]: unknown };
type UiLogEntry = {
	id: number;
	message: string;
	level: LogLevel;
	timestamp: number;
	data: LogData;
	style: LogStyle;
	app: string;
};
function transformLog(newLog: LogEntry) {
	let isString;
	try {
		if (typeof newLog.message == 'object') {
			isString = false;
		} else {
			isString = true;
			if (typeof JSON.parse(newLog.message) === 'object') {
				isString = false;
			} else {
				isString = true;
			}
		}
	} catch {
		isString = true;
	}
	let app = '-';
	const streamParts = newLog.log_stream_name.split('/');
	if (streamParts.length > 0) {
		app = streamParts[1];
	}
	if (isString) {
		const level = (newLog.message.match(/(INFO|WARN|ERROR|DEBUG|TRACE)/)?.[0] ??
			'UNKNOWN') as LogLevel;
		return {
			app,
			timestamp: newLog.timestamp,
			level,
			message: newLog.message,
			data: {
				app,
				timestamp:
					typeof newLog.timestamp == 'number' || typeof newLog.timestamp == 'string'
						? format(new Date(newLog.timestamp), 'yyyy-MM-dd HH:mm:ss.SSS')
						: newLog.timestamp,
				level,
				message: newLog.message
			},
			style: logStyle(level)
		};
	} else {
		const logData = JSON.parse(newLog.message);
		const level = logData?.level?.match(/(INFO|WARN|ERROR|DEBUG|TRACE)/)?.[0] ?? 'UNKNOWN';
		return {
			app,
			timestamp: newLog.timestamp,
			level,
			message: logData.message,
			data: logData,
			style: logStyle(level)
		};
	}
}
function logStyle(level: LogLevel): LogStyle {
	switch (level) {
		case 'INFO':
			return {
				bg: 'bg-cyan-900',
				hover: 'hover:bg-cyan-800',
				active: '!bg-cyan-700'
			};
		case 'WARN':
			return {
				bg: 'bg-amber-900',
				hover: 'hover:bg-amber-800',
				active: '!bg-amber-700'
			};
		case 'ERROR':
			return {
				bg: 'bg-red-900',
				hover: 'hover:bg-red-800',
				active: '!bg-red-700'
			};
		case 'TRACE':
			return {
				bg: 'bg-fuchsia-900',
				hover: 'hover:bg-fuchsia-800',
				active: '!bg-fuchsia-700'
			};
		case 'DEBUG':
			return {
				bg: 'bg-emerald-900',
				hover: 'hover:bg-emerald-800',
				active: '!bg-emerald-700'
			};
		case 'UNKNOWN':
			return {
				bg: 'bg-gray-900',
				hover: 'hover:bg-gray-800',
				active: '!bg-gray-700'
			};
	}
}
const createLogStore = () => {
	const selectedLog = writable<LogData | undefined>(undefined);
	const timerange = writable<Timerange>({
		type: 'relative',
		amount: 30,
		unit: 'minutes'
	});

	const filterString = writable<string>('');

	const storeState = writable<{
		showLogDetails: boolean;
		message: string | undefined;
		isLookingForLogs: boolean;
		searchStatus: 'success' | 'error' | 'aborted' | undefined;
		logs: UiLogEntry[];
	}>({
		showLogDetails: false,
		message: undefined,
		isLookingForLogs: false,
		searchStatus: undefined,
		logs: []
	});

	function processLogs(newLogs: LogEntry[]) {
		storeState.update((state) => {
			let startIndex = state.logs.length;
			const newLogArr = newLogs.map((log) => ({ id: startIndex++, ...transformLog(log) }));
			return {
				...state,
				logs: [...state.logs, ...newLogArr]
			};
		});
	}

	listen<LogEntry[]>('new-log-found', (event) => processLogs(event.payload));

	listen<string>('find-logs-success', (event) => {
		storeState.update((state) => {
			return {
				...state,
				isLookingForLogs: false,
				searchStatus: 'success',
				message: event.payload
			};
		});
	});
	listen<string>('find-logs-error', (event) => {
		storeState.update((state) => {
			return {
				...state,
				isLookingForLogs: false,
				searchStatus: 'error',
				message: event.payload
			};
		});
	});

	listen<string>('find-logs-message', (event) => {
		console.log('message:', event.payload);
		storeState.update((state) => {
			return {
				...state,
				message: event.payload
			};
		});
	});

	const timerangeToPartial = (
		timerange: Timerange
	): { startTimestamp: number; endTimestamp: number } => {
		switch (timerange.type) {
			case 'absolute':
				return {
					startTimestamp: timerange.from.getTime(),
					endTimestamp: timerange.to.getTime()
				};
			case 'relative':
				return {
					startTimestamp: new Date().getTime() - timerange.amount * unitToMs(timerange.unit),
					endTimestamp: new Date().getTime()
				};
		}
	};

	const unitToMs = (unit: TimeUnit): number => {
		switch (unit) {
			case 'minutes':
				return 60 * 1000;
			case 'hours':
				return 60 * 60 * 1000;
			case 'days':
				return 24 * 60 * 60 * 1000;
		}
	};

	const search = (apps: string[], env: AwsEnv) => {
		invoke('find_logs', {
			apps,
			env,
			...timerangeToPartial(get(timerange)),
			filter: get(filterString)
		});
		storeState.update((state) => {
			return {
				...state,
				isLookingForLogs: true,
				message: 'Search in progress...',
				logs: [],
				showLogDetails: false,
				searchStatus: undefined
			};
		});
	};
	const dumpLogs = (apps: string[], env: AwsEnv) => {
		invoke('find_logs', {
			apps,
			env,
			...timerangeToPartial(get(timerange)),
			filter: get(filterString),
			filename: `${apps.join('_')}-${env?.toLowerCase()}`
		});
		storeState.update((state) => {
			return {
				...state,
				isLookingForLogs: true,
				searchError: undefined,
				logs: [],
				showLogDetails: false,
				searchStatus: undefined
			};
		});
	};

	const abort = (reason: string) => {
		invoke('abort_find_logs', { reason });
		storeState.update((state) => {
			return {
				...state,
				isLookingForLogs: false,
				searchStatus: state.isLookingForLogs ? 'aborted' : state.searchStatus,
				message: 'Aborted'
			};
		});
	};

	const showLog = (log: UiLogEntry) => {
		selectedLog.update((c) => {
			const newValue = c === log.data ? undefined : log.data;
			storeState.update((state) => {
				return { ...state, showLogDetails: !!newValue };
			});

			return newValue;
		});
	};
	return {
		showLog,
		abort,
		search,
		dumpLogs,
		selectedLog,
		timerange,
		filterString,
		storeState
	};
};

export const logStore = createLogStore();
listen('logged-out', () => {
	logStore.selectedLog.set(undefined);
	logStore.timerange.set({
		type: 'relative',
		amount: 30,
		unit: 'minutes'
	});
	logStore.filterString.set('');
	logStore.storeState.set({
		showLogDetails: false,
		message: undefined,
		isLookingForLogs: false,
		searchStatus: undefined,
		logs: []
	});
});
