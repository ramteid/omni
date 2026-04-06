<script lang="ts">
    import * as Dialog from '$lib/components/ui/dialog'
    import { Button } from '$lib/components/ui/button'
    import { Input } from '$lib/components/ui/input'
    import { Label } from '$lib/components/ui/label'
    import { AuthType, ServiceProvider, SourceType } from '$lib/types'
    import { toast } from 'svelte-sonner'

    interface Props {
        open: boolean
        onSuccess?: () => void
        onCancel?: () => void
    }

    let { open = $bindable(false), onSuccess, onCancel }: Props = $props()

    let sourceName = $state('Paperless-ngx')
    let url = $state('')
    let apiKey = $state('')
    let isSubmitting = $state(false)

    async function handleSubmit() {
        if (!url.trim()) {
            toast.error('Paperless-ngx URL is required')
            return
        }
        if (!apiKey.trim()) {
            toast.error('API key is required')
            return
        }

        isSubmitting = true

        try {
            const config = {
                url: url.trim().replace(/\/$/, ''),
                sync_enabled: true,
            }

            // 1. Create the source record
            const sourceResponse = await fetch('/api/sources', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    name: sourceName.trim() || 'Paperless-ngx',
                    sourceType: SourceType.PAPERLESS_NGX,
                    config,
                }),
            })

            if (!sourceResponse.ok) {
                const text = await sourceResponse.text()
                throw new Error(`Failed to create Paperless-ngx source: ${text}`)
            }

            const source = await sourceResponse.json()

            // 2. Persist the API key via the encrypted service-credentials API.
            const credentialsResponse = await fetch('/api/service-credentials', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    sourceId: source.id,
                    provider: ServiceProvider.PAPERLESS_NGX,
                    authType: AuthType.API_KEY,
                    credentials: { api_key: apiKey.trim() },
                }),
            })

            if (!credentialsResponse.ok) {
                const text = await credentialsResponse.text()
                throw new Error(`Failed to save API key: ${text}`)
            }

            toast.success('Paperless-ngx connected successfully!')
            open = false
            resetForm()

            if (onSuccess) {
                onSuccess()
            }
        } catch (err: any) {
            console.error('Error setting up Paperless-ngx:', err)
            toast.error(err.message || 'Failed to connect Paperless-ngx')
        } finally {
            isSubmitting = false
        }
    }

    function resetForm() {
        sourceName = 'Paperless-ngx'
        url = ''
        apiKey = ''
    }

    function handleCancel() {
        open = false
        resetForm()
        if (onCancel) {
            onCancel()
        }
    }
</script>

<Dialog.Root bind:open>
    <Dialog.Content class="max-w-lg">
        <Dialog.Header>
            <Dialog.Title>Connect Paperless-ngx</Dialog.Title>
            <Dialog.Description>
                Index documents from your Paperless-ngx document management system. The API key is
                stored encrypted and never leaves the server.
            </Dialog.Description>
        </Dialog.Header>

        <div class="space-y-4">
            <!-- Source name -->
            <div class="space-y-1.5">
                <Label for="paperless-name">Source name</Label>
                <Input
                    id="paperless-name"
                    bind:value={sourceName}
                    placeholder="e.g. My Documents"
                    disabled={isSubmitting} />
            </div>

            <!-- URL -->
            <div class="space-y-1.5">
                <Label for="paperless-url">Paperless-ngx URL</Label>
                <Input
                    id="paperless-url"
                    bind:value={url}
                    placeholder="http://paperless:8000"
                    autocomplete="url"
                    disabled={isSubmitting}
                    required />
                <p class="text-muted-foreground text-xs">
                    The base URL of your Paperless-ngx instance, without a trailing slash.
                </p>
            </div>

            <!-- API Key -->
            <div class="space-y-1.5">
                <Label for="paperless-apikey">API Key</Label>
                <Input
                    id="paperless-apikey"
                    type="password"
                    bind:value={apiKey}
                    placeholder="Your Paperless-ngx API key"
                    autocomplete="current-password"
                    disabled={isSubmitting}
                    required />
                <p class="text-muted-foreground text-xs">
                    Generate an API token in Paperless-ngx under
                    <strong>Settings → API Auth Tokens</strong>.
                </p>
            </div>
        </div>

        <Dialog.Footer>
            <Button
                variant="outline"
                onclick={handleCancel}
                disabled={isSubmitting}
                class="cursor-pointer">
                Cancel
            </Button>
            <Button onclick={handleSubmit} disabled={isSubmitting} class="cursor-pointer">
                {isSubmitting ? 'Connecting…' : 'Connect'}
            </Button>
        </Dialog.Footer>
    </Dialog.Content>
</Dialog.Root>
