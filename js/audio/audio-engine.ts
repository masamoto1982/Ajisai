










type WaveformType = 'sine' | 'square' | 'sawtooth' | 'triangle';

interface Envelope {
    attack: number;
    decay: number;
    sustain: number;
    release: number;
}

interface AudioStructure {
    type: 'tone' | 'rest' | 'seq' | 'sim';
    frequency?: number;
    duration?: number;
    children?: AudioStructure[];
    envelope?: Envelope;
    waveform?: WaveformType;
}

interface PlayCommand {
    type: 'play';
    structure: AudioStructure;
}


const DEFAULT_ENVELOPE: Envelope = {
    attack: 0.01,
    decay: 0.0,
    sustain: 1.0,
    release: 0.01,
};

export class AudioEngine {
    private audioContext: AudioContext | null = null;
    private isInitialized = false;
    private slotDuration = 0.5;
    private currentGain = 1.0;
    private currentPan = 0.0;

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


        if (this.audioContext.state === 'suspended') {
            await this.audioContext.resume();
        }

        const startTime = this.audioContext.currentTime;
        await this.playStructure(command.structure, startTime);
    }

    private async playStructure(
        structure: AudioStructure,
        startTime: number,
        inheritedEnvelope?: Envelope,
        inheritedWaveform?: WaveformType
    ): Promise<number> {
        if (!this.audioContext) return startTime;


        const envelope = structure.envelope || inheritedEnvelope || DEFAULT_ENVELOPE;
        const waveform = structure.waveform || inheritedWaveform || 'sine';

        switch (structure.type) {
            case 'tone':
                const toneDuration = (structure.duration || 1) * this.slotDuration;
                this.playTone(structure.frequency!, startTime, toneDuration, envelope, waveform);
                return startTime + toneDuration;

            case 'rest':
                const restDuration = (structure.duration || 1) * this.slotDuration;
                return startTime + restDuration;

            case 'seq':

                let seqTime = startTime;
                for (const child of structure.children || []) {
                    seqTime = await this.playStructure(child, seqTime, envelope, waveform);
                }
                return seqTime;

            case 'sim':

                const endTimes = await Promise.all(
                    (structure.children || []).map(child =>
                        this.playStructure(child, startTime, envelope, waveform)
                    )
                );
                return Math.max(...endTimes, startTime);

            default:
                console.warn('Unknown structure type:', (structure as any).type);
                return startTime;
        }
    }

    private playTone(
        frequency: number,
        startTime: number,
        duration: number,
        envelope: Envelope,
        waveform: WaveformType
    ): void {
        if (!this.audioContext) return;

        const oscillator = this.audioContext.createOscillator();
        const gainNode = this.audioContext.createGain();
        const panNode = this.audioContext.createStereoPanner();


        oscillator.connect(gainNode);
        gainNode.connect(panNode);
        panNode.connect(this.audioContext.destination);

        oscillator.frequency.setValueAtTime(frequency, startTime);
        oscillator.type = waveform;


        panNode.pan.setValueAtTime(this.currentPan, startTime);


        const { attack, decay, sustain, release } = envelope;
        const peakLevel = 0.3 * this.currentGain;
        const sustainLevel = peakLevel * sustain;


        const effectiveDuration = Math.max(duration - release, attack + decay);


        gainNode.gain.setValueAtTime(0, startTime);
        gainNode.gain.linearRampToValueAtTime(peakLevel, startTime + attack);


        if (decay > 0) {
            gainNode.gain.linearRampToValueAtTime(sustainLevel, startTime + attack + decay);
        }



        const releaseStart = startTime + effectiveDuration;
        gainNode.gain.setValueAtTime(sustainLevel, releaseStart);
        gainNode.gain.exponentialRampToValueAtTime(0.001, releaseStart + release);

        oscillator.start(startTime);
        oscillator.stop(startTime + duration + release);
    }

    updateSlotDuration(seconds: number): void {
        this.slotDuration = seconds;
    }

    extractSlotDuration(): number {
        return this.slotDuration;
    }

    updateGain(value: number): void {
        this.currentGain = Math.max(0, Math.min(1, value));
        console.log(`Audio gain set to ${this.currentGain}`);
    }

    extractGain(): number {
        return this.currentGain;
    }

    updatePan(value: number): void {
        this.currentPan = Math.max(-1, Math.min(1, value));
        console.log(`Audio pan set to ${this.currentPan}`);
    }

    extractPan(): number {
        return this.currentPan;
    }
}

export const AUDIO_ENGINE = new AudioEngine();
