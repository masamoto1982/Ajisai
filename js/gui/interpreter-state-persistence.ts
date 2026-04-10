import {
    createPersistenceService,
    type PersistenceService,
    type PersistenceServiceOptions,
    type InterpreterState,
    persistenceServiceUtils
} from '../application/persistence-service';

export type PersistenceCallbacks = PersistenceServiceOptions;
export type Persistence = PersistenceService;

export const createPersistence = (callbacks: PersistenceCallbacks): Persistence =>
    createPersistenceService(callbacks);

export const persistenceUtils = persistenceServiceUtils;

export type { InterpreterState };
