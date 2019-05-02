// import axios from 'axios';
import { MatcherConfigDto } from '@/generated/dto';

export async function getConfig(): Promise<MatcherConfigDto> {
    const response = await fetch('/api/config');
    // const response = await fetch('http://127.0.0.1:4748/api/config');
    return response.json();
}
