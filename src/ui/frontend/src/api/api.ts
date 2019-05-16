import axios from 'axios';
import { MatcherConfigDto, SendEventRequestDto, ProcessedEventDto } from '@/generated/dto';

const axiosInstance = axios.create({
    baseURL: 'http://127.0.0.1:4748',
});

export async function getConfig(): Promise<MatcherConfigDto> {
    const response = await axiosInstance.get<MatcherConfigDto>('/api/config');
    return response.data;
}

export async function postSendEvent(request: SendEventRequestDto): Promise<ProcessedEventDto> {
    const response = await axiosInstance.post<ProcessedEventDto>('/api/send_event', request);
    return response.data;
}
