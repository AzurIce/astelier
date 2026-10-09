import type { ModelProfile } from '../generation/profiles'

export interface BackendConfig {
	id: string
	name: string
	kind: 'opfs' | 'http'
	baseUrl?: string
}

export interface ProviderDescriptor {
	id: string
	name: string
	models: string[]
	profiles: Record<string, ModelProfile>
}

export interface BackendConnection extends BackendConfig {
	status: 'connecting' | 'online' | 'offline'
	error: string | null
	providerError: string | null
	providers: ProviderDescriptor[]
	revision: number
}

export interface GraphLocation { backendId: string; id: string }
export interface ProviderLocation { backendId: string; providerId: string; modelId: string }
