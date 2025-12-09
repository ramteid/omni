import type { InputMode } from '$lib/components/user-input.svelte'

export interface UserPreferences {
    inputMode: InputMode
}

export const DEFAULT_PREFERENCES: UserPreferences = {
    inputMode: 'chat',
}

export const STORAGE_KEY = 'omni-user-preferences'
