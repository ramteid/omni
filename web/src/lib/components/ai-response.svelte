<script lang="ts">
    import { Card, CardContent, CardHeader, CardTitle } from '$lib/components/ui/card/index.js'
    import { Loader2, Sparkles, Search, ChevronDown, ChevronUp } from '@lucide/svelte'
    import { marked } from 'marked'

    interface Props {
        query: string
        sources?: string[]
        contentTypes?: string[]
    }

    let { query, sources, contentTypes }: Props = $props()

    let isLoading = $state(true)
    let answer = $state('')
    let error = $state<string | null>(null)
    let isExpanded = $state(false)
    let shouldShowExpandButton = $state(false)
    let contentRef: HTMLDivElement | undefined = $state()
    let searchProgress = $state('')

    // Start streaming AI-first answer automatically when component mounts
    $effect(() => {
        if (query.trim()) {
            streamAIFirstAnswer()
        }
    })

    async function streamAIFirstAnswer() {
        isLoading = true
        answer = ''
        error = null
        searchProgress = ''

        try {
            const response = await fetch('/api/ask', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    query: query.trim(),
                    sources,
                    content_types: contentTypes,
                }),
            })

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`)
            }

            const reader = response.body?.getReader()
            if (!reader) {
                throw new Error('No response body reader available')
            }

            const decoder = new TextDecoder()
            let buffer = ''

            while (true) {
                const { done, value } = await reader.read()

                if (done) {
                    break
                }

                const chunk = decoder.decode(value, { stream: true })
                buffer += chunk

                // Look for search progress indicators in the stream
                if (chunk.includes('Searching for:')) {
                    const searchMatch = chunk.match(/Searching for: ([^\n]+)/)
                    if (searchMatch) {
                        searchProgress = `🔍 ${searchMatch[1]}`
                    }
                } else if (chunk.includes('Found ') && chunk.includes(' documents')) {
                    const foundMatch = chunk.match(/Found (\d+) documents/)
                    if (foundMatch) {
                        searchProgress = `📄 Found ${foundMatch[1]} documents`
                    }
                }

                answer = buffer
                isLoading = false
            }

            // Clear search progress when done
            searchProgress = ''
        } catch (err) {
            console.error('Error streaming AI-first answer:', err)
            error = 'Failed to generate AI answer. Please try again.'
            isLoading = false
            searchProgress = ''
        }
    }

    // Format answer with markdown
    async function formatAnswerWithMarkdown(text: string): Promise<string> {
        return await marked.parse(text)
    }

    let formattedAnswer = $state('')

    // Update formatted answer when answer changes
    $effect(() => {
        if (answer) {
            formatAnswerWithMarkdown(answer).then((html) => {
                formattedAnswer = html
            })
        } else {
            formattedAnswer = ''
        }
    })

    // Check if content exceeds max height after answer is rendered
    $effect(() => {
        if (formattedAnswer && contentRef) {
            // Use a small delay to ensure DOM is updated
            setTimeout(() => {
                if (contentRef) {
                    const scrollHeight = contentRef.scrollHeight
                    const maxHeight = 400 // Larger max height for AI-first mode
                    shouldShowExpandButton = scrollHeight > maxHeight
                }
            }, 10)
        }
    })
</script>

<Card class="mb-8 border-0 bg-gradient-to-br from-blue-50 via-indigo-50 to-purple-50 shadow-lg">
    <CardHeader>
        <CardTitle class="flex items-center gap-2 text-xl">
            <Sparkles class="h-6 w-6 text-indigo-600" />
            AI Assistant
        </CardTitle>
    </CardHeader>
    <CardContent class="space-y-4">
        {#if isLoading}
            <div class="flex items-center gap-2 text-gray-600">
                <Loader2 class="h-4 w-4 animate-spin" />
                Thinking about your question...
            </div>
        {:else if error}
            <div class="text-red-600">
                {error}
            </div>
        {:else if answer}
            <div>
                <div class="relative">
                    <div
                        bind:this={contentRef}
                        class="prose prose-sm prose-blue max-w-none overflow-hidden transition-all duration-300"
                        class:max-h-[400px]={!isExpanded && shouldShowExpandButton}>
                        <!-- Use innerHTML to render markdown -->
                        {@html formattedAnswer}
                    </div>

                    {#if !isExpanded && shouldShowExpandButton}
                        <div
                            class="pointer-events-none absolute -right-6 bottom-0 -left-6 h-20 bg-gradient-to-t from-purple-50 to-transparent">
                        </div>
                    {/if}
                </div>

                {#if shouldShowExpandButton}
                    <button
                        onclick={() => (isExpanded = !isExpanded)}
                        class="mt-4 flex items-center gap-1 text-sm text-indigo-600 transition-colors hover:text-indigo-700">
                        {#if isExpanded}
                            <ChevronUp class="h-4 w-4" />
                            Show less
                        {:else}
                            <ChevronDown class="h-4 w-4" />
                            Show more
                        {/if}
                    </button>
                {/if}
            </div>
        {:else}
            <div class="text-gray-500">No answer generated.</div>
        {/if}

        <!-- Search Progress Indicator -->
        {#if searchProgress}
            <div
                class="flex items-center gap-2 rounded-lg bg-indigo-100 p-3 text-sm text-indigo-700">
                <Search class="h-4 w-4 animate-pulse" />
                {searchProgress}
            </div>
        {/if}
    </CardContent>
</Card>
