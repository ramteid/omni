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
    let baseUrl = $state('')
    let apiKey = $state('')

    let isSubmitting = $state(false)

    async function handleSubmit() {
        if (!baseUrl.trim()) {
            toast.error('Paperless-ngx URL is required')
            return
        }
        if (!apiKey.trim()) {
            toast.error('API key is required')
            return
        }

        // Basic URL validation
        try {
            new URL(baseUrl.trim())
        } catch {
            toast.error('Please enter a valid URL (e.g. http://localhost:8000)')
            return
        }

        isSubmitting = true

        try {
            const config = {
                base_url: baseUrl.trim().replace(/\/$/, ''),
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
                throw new Error(`Failed to create source: ${text}`)
            }

            const source = await sourceResponse.json()

            // 2. Persist the API key via the encrypted service-credentials API
            const credentialsResponse = await fetch('/api/service-credentials', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    sourceId: source.id,
                    provider: ServiceProvider.PAPERLESS_NGX,
                    authType: AuthType.API_KEY,
                    credentials: {
                        api_key: apiKey.trim(),
                    },
                }),
            })

            if (!credentialsResponse.ok) {
                const text = await credentialsResponse.text()
                throw new Error(`Failed to save credentials: ${text}`)
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
        baseUrl = ''
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
                Index documents and their OCR content from your paperless-ngx instance. The API key
                is stored encrypted and never leaves the server.
            </Dialog.Description>
        </Dialog.Header>

        <div class="space-y-4">
            <!-- Source name -->
            <div class="space-y-1.5">
                <Label for="paperless-name">Source name</Label>
                <Input
                    id="paperless-name"
                    bind:value={sourceName}
                    placeholder="e.g. Home Paperless"
                    disabled={isSubmitting} />
            </div>

            <!-- Instance URL -->
            <div class="space-y-1.5">
                <Label for="paperless-url">Paperless-ngx URL</Label>
                <Input
                    id="paperless-url"
                    bind:value={baseUrl}
                    placeholder="http://localhost:8000"
                    disabled={isSubmitting}
                    required />
                <p class="text-muted-foreground text-xs">
                    The base URL of your paperless-ngx instance, without a trailing slash.
                </p>
            </div>

            <!-- API key -->
            <div class="space-y-1.5">
                <Label for="paperless-key">API key</Label>
                <Input
                    id="paperless-key"
                    type="password"
                    bind:value={apiKey}
                    placeholder="Your paperless-ngx API token"
                    disabled={isSubmitting}
                    required />
                <p class="text-muted-foreground text-xs">
                    Generate an API token in paperless-ngx under <strong
                        >Settings → API Tokens</strong
                    >.
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
