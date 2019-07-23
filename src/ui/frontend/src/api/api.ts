import axios from 'axios';
import { MatcherConfigDto, SendEventRequestDto, ProcessedEventDto } from '@/generated/dto';

const axiosInstance = axios.create({
    baseURL: '/neteye/tornado/backend/api',
});

export async function getConfig(): Promise<MatcherConfigDto> {
    const response = await axiosInstance.get<MatcherConfigDto>('/config');
    return response.data;
}

export async function postSendEvent(request: SendEventRequestDto): Promise<ProcessedEventDto> {
    const response = await axiosInstance.post<ProcessedEventDto>('/send_event', request);
    return response.data;
}
