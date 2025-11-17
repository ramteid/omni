import { hash, verify } from '@node-rs/argon2'
import { randomBytes } from 'crypto'

const ARGON2_CONFIG = {
    memoryCost: 65536,
    timeCost: 3,
    outputLen: 32,
    parallelism: 1,
}

export async function hashPassword(password: string): Promise<string> {
    return hash(password, ARGON2_CONFIG)
}

export async function verifyPassword(hash: string, password: string): Promise<boolean> {
    return verify(hash, password)
}

export function generateSecurePassword(length: number = 12): string {
    const charset = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*'
    const charsetLength = charset.length
    const randomBytesArray = randomBytes(length)

    let password = ''
    for (let i = 0; i < length; i++) {
        const randomIndex = randomBytesArray[i] % charsetLength
        password += charset[randomIndex]
    }

    return password
}
