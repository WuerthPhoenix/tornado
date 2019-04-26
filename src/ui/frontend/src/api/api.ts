import axios from 'axios';
import { MatcherConfigDto } from '@/generated/dto';

const axiosInstance = axios.create({});

export async function getConfigAsynch(): Promise<MatcherConfigDto> {
    const response = await axiosInstance.get<MatcherConfigDto>('/api/config');
    return response.data;
}


export async function getConfig(): Promise<MatcherConfigDto> {
    return axiosInstance.get<MatcherConfigDto>('/api/config')
        .then((response) => response.data);
}
