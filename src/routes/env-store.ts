import { invoke } from '@tauri-apps/api';
import { writable } from 'svelte/store';
import { Env, type UserConfig } from './types';

export const currentEnv = writable<Env>(Env.DEV);
