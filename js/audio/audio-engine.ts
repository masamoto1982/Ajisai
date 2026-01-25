// js/audio/audio-engine.ts
//
// Audio playback engine for Ajisai's music DSL.
// Recursively processes SEQ/SIM structures and plays audio via Web Audio API.
//
// AudioStructure format:
// - tone: { type: 'tone', frequency: number, duration: number }
// - rest: { type: 'rest', duration: number }
// - seq: { type: 'seq', children: AudioStructure[] }
// - sim: { type: 'sim', children: AudioStructure[] }

interface AudioStructure {
    type: 'tone' | 'rest' | 'seq' | 'sim';
    frequency?: number;
    duration?: number;
    children?: AudioStructure[];
}

interface PlayCommand {
    type: 'play';
    structure: AudioStructure;
}

export class AudioEngine {
    private audioContext: AudioContext | null = null;
    private isInitialized = false;
    private slotDuration = 0.5; // 0.5 seconds per slot

    async init(): Promise<void> {
        if (this.isInitialized) return;

        try {
            this.audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
            this.isInitialized = true;
            console.log('Audio engine initialized');
        } catch (error) {
            console.error('Failed to initialize audio engine:', error);
        }
    }

    async playAudioCommand(command: PlayCommand): Promise<void> {
        if (!this.audioContext) {
            await this.init();
        }

        if (!this.audioContext) {
            console.error('Audio context not available');
            return;
        }

        // Resume audio context if suspended (required by some browsers)
        if (this.audioContext.state === 'suspended') {
            await this.audioContext.resume();
        }

        const startTime = this.audioContext.currentTime;
        await this.playStructure(command.structure, startTime);
    }

    private async playStructure(structure: AudioStructure, startTime: number): Promise<number> {
        if (!this.audioContext) return startTime;

        switch (structure.type) {
            case 'tone':
                const toneDuration = (structure.duration || 1) * this.slotDuration;
                this.playTone(structure.frequency!, startTime, toneDuration);
                return startTime + toneDuration;

            case 'rest':
                const restDuration = (structure.duration || 1) * this.slotDuration;
                return startTime + restDuration;

            case 'seq':
                // Sequential: play children one after another
                let seqTime = startTime;
                for (const child of structure.children || []) {
                    seqTime = await this.playStructure(child, seqTime);
                }
                return seqTime;

            case 'sim':
                // Simultaneous: play all children at the same time
                const endTimes = await Promise.all(
                    (structure.children || []).map(child =>
                        this.playStructure(child, startTime)
                    )
                );
                return Math.max(...endTimes, startTime);

            default:
                console.warn('Unknown structure type:', (structure as any).type);
                return startTime;
        }
    }

    private playTone(frequency: number, startTime: number, duration: number): void {
        if (!this.audioContext) return;

        const oscillator = this.audioContext.createOscillator();
        const gainNode = this.audioContext.createGain();

        oscillator.connect(gainNode);
        gainNode.connect(this.audioContext.destination);

        oscillator.frequency.setValueAtTime(frequency, startTime);
        oscillator.type = 'sine';

        // Envelope for smooth sound
        gainNode.gain.setValueAtTime(0, startTime);
        gainNode.gain.linearRampToValueAtTime(0.3, startTime + 0.01);
        gainNode.gain.exponentialRampToValueAtTime(0.01, startTime + duration);

        oscillator.start(startTime);
        oscillator.stop(startTime + duration);
    }

    setSlotDuration(seconds: number): void {
        this.slotDuration = seconds;
    }

    getSlotDuration(): number {
        return this.slotDuration;
    }
}

export const AUDIO_ENGINE = new AudioEngine();
