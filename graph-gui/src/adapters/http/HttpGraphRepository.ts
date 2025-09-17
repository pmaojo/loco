import { RawGraphSnapshot } from '../../core/models/Graph';
import { GraphRepository } from '../../core/ports/GraphRepository';

export class HttpGraphRepository implements GraphRepository {
  constructor(private readonly baseUrl = '') {}

  async fetchGraph(): Promise<RawGraphSnapshot> {
    const response = await fetch(`${this.baseUrl}/__loco/graph`, {
      headers: { Accept: 'application/json' },
    });

    if (!response.ok) {
      throw new Error(`Unable to load graph (status ${response.status})`);
    }

    return (await response.json()) as RawGraphSnapshot;
  }
}
