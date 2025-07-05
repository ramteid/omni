<script>
    import { onMount } from 'svelte'

    let status = 'processing'
    let message = 'Processing your magic link...'

    onMount(() => {
        // This page should redirect automatically via the server load function
        // If we get here, there might be an error
        setTimeout(() => {
            if (status === 'processing') {
                status = 'error'
                message = 'Something went wrong. Please try again.'
            }
        }, 5000)
    })
</script>

<div class="flex min-h-screen items-center justify-center bg-gray-50">
    <div class="w-full max-w-md space-y-8">
        <div class="text-center">
            <h2 class="mt-6 text-3xl font-bold text-gray-900">
                {#if status === 'processing'}
                    Signing you in...
                {:else}
                    Authentication Error
                {/if}
            </h2>
            <p class="mt-2 text-sm text-gray-600">
                {message}
            </p>
        </div>

        {#if status === 'processing'}
            <div class="flex justify-center">
                <div class="h-8 w-8 animate-spin rounded-full border-b-2 border-blue-600"></div>
            </div>
        {:else}
            <div class="text-center">
                <a href="/login" class="text-sm text-blue-600 hover:text-blue-500">
                    Return to login
                </a>
            </div>
        {/if}
    </div>
</div>
