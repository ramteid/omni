<script>
    import { page } from '$app/stores'

    $: email = $page.url.searchParams.get('email') || ''
</script>

<div class="flex min-h-screen items-center justify-center bg-gray-50">
    <div class="w-full max-w-md space-y-8">
        <div class="text-center">
            <div
                class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-green-100"
            >
                <svg
                    class="h-6 w-6 text-green-600"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                >
                    <path
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        stroke-width="2"
                        d="M5 13l4 4L19 7"
                    />
                </svg>
            </div>
            <h2 class="mt-6 text-3xl font-bold text-gray-900">Check your email</h2>
            <p class="mt-2 text-sm text-gray-600">
                We've sent a secure login link to
                {#if email}
                    <span class="font-medium">{email}</span>
                {:else}
                    your email address
                {/if}
            </p>
        </div>

        <div class="rounded-md bg-blue-50 p-4">
            <div class="text-sm text-blue-700">
                <p class="font-medium">What's next?</p>
                <ul class="mt-2 list-inside list-disc space-y-1">
                    <li>Check your email inbox (and spam folder)</li>
                    <li>Click the "Sign In" button in the email</li>
                    <li>You'll be automatically logged in</li>
                </ul>
            </div>
        </div>

        <div class="space-y-2 text-center">
            <p class="text-sm text-gray-500">The link will expire in 15 minutes for security.</p>
            <div class="flex justify-center space-x-4">
                <a
                    href="/auth/request-magic-link"
                    class="text-sm text-blue-600 hover:text-blue-500"
                >
                    Send another link
                </a>
                <a href="/login" class="text-sm text-blue-600 hover:text-blue-500">
                    Back to login
                </a>
            </div>
        </div>
    </div>
</div>
